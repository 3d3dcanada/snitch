"""Read, write, strip and stamp image metadata.

Three separate jobs that people usually conflate:

    snitch      read everything and say plainly what is there
    credit      write credit in, and optionally stamp it into the pixels
    no-comment  take metadata out

STRIPPING IS SEGMENT SURGERY, NOT RE-ENCODING. Dropping JPEG APPn/COM markers and keeping only
the required PNG chunks leaves the decoded pixels byte-identical. Re-encoding to "clear" metadata
throws away image quality for no reason, and this tool will not do it.

STAMPING DOES re-encode, because it has to: it changes pixels. That is stated at the call site
and in the CLI output, because a user deserves to know which operation costs them quality.
"""

import io
import json
import os
import shutil
import struct
import subprocess

# JPEG markers that never carry image data. FFD8 start, FFD9 end, FFDA is the scan.
_JPEG_SKIPPABLE = set(range(0xE0, 0xF0)) | {0xFE}          # APP0..APP15 and COM
_PNG_REQUIRED = {b"IHDR", b"PLTE", b"IDAT", b"IEND", b"tRNS", b"gAMA", b"cHRM", b"sRGB"}


class ToolMissing(RuntimeError):
    pass


def _run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def have(tool):
    return shutil.which(tool) is not None


def require(tool, why):
    if not have(tool):
        raise ToolMissing(f"{tool} is not installed, and {why}.\n"
                          f"  Debian/Ubuntu:  sudo apt install {tool}\n"
                          f"  macOS:          brew install {tool}")


# --------------------------------------------------------------------------------------------
# inspect
# --------------------------------------------------------------------------------------------

def read_metadata(path):
    """Everything exiftool can see, as a dict."""
    require("exiftool", "reading metadata needs it")
    r = _run(["exiftool", "-j", "-G", "-n", "-a", "-u", path])
    if r.returncode or not r.stdout.strip():
        return {}
    try:
        return json.loads(r.stdout)[0]
    except Exception:
        return {}


def read_c2pa(path, c2patool=None):
    """The C2PA manifest, or None. Never raises: absence is the common case."""
    tool = c2patool or shutil.which("c2patool") or os.path.expanduser("~/.cargo/bin/c2patool")
    if not os.path.exists(tool) if os.path.isabs(tool) else not shutil.which(tool):
        return None
    r = _run([tool, path])
    if r.returncode or not r.stdout.strip():
        return None
    try:
        return json.loads(r.stdout)
    except Exception:
        return None


CREDIT_FIELDS = [
    ("Creator", ["XMP:Creator", "IPTC:By-line", "EXIF:Artist"]),
    ("Credit", ["XMP:Credit", "IPTC:Credit"]),
    ("Copyright", ["XMP:Rights", "IPTC:CopyrightNotice", "EXIF:Copyright"]),
    ("Usage terms", ["XMP:UsageTerms"]),
    ("Rights URL", ["XMP:WebStatement"]),
    ("Licensor", ["XMP:LicensorName", "XMP:LicensorURL"]),
    ("Contact", ["XMP:CreatorWorkEmail", "XMP:CreatorWorkURL"]),
    ("Title", ["XMP:Title", "IPTC:ObjectName"]),
    ("Description", ["XMP:Description", "IPTC:Caption-Abstract"]),
    ("Keywords", ["XMP:Subject", "IPTC:Keywords"]),
]

GPS_FIELDS = ["EXIF:GPSLatitude", "EXIF:GPSLongitude", "Composite:GPSPosition",
              "EXIF:GPSAltitude"]

CAMERA_FIELDS = [("Camera", ["EXIF:Make", "EXIF:Model"]),
                 ("Lens", ["EXIF:LensModel", "EXIF:LensInfo"]),
                 ("Taken", ["EXIF:DateTimeOriginal", "EXIF:CreateDate"]),
                 ("Software", ["EXIF:Software", "XMP:CreatorTool"])]


def _first(meta, keys):
    for k in keys:
        v = meta.get(k)
        if v not in (None, "", []):
            return v
    return None


def inspect(path, c2patool=None):
    """A structured report. The CLI formats it; the data is here so other things can use it."""
    meta = read_metadata(path)
    c2pa = read_c2pa(path, c2patool)

    gps = {k: meta[k] for k in GPS_FIELDS if k in meta}
    credit = {}
    for label, keys in CREDIT_FIELDS:
        v = _first(meta, keys)
        if v is not None:
            credit[label] = v
    camera = {}
    for label, keys in CAMERA_FIELDS:
        v = _first(meta, keys)
        if v is not None:
            camera[label] = v

    ai = None
    if c2pa:
        active = c2pa.get("active_manifest")
        man = (c2pa.get("manifests") or {}).get(active, {})
        for a in man.get("assertions", []):
            if "action" in (a.get("label") or ""):
                for act in (a.get("data") or {}).get("actions", []):
                    src = (act.get("digitalSourceType") or "")
                    if "trainedAlgorithmicMedia" in src or "compositeWithTrainedAlgorithmic" in src:
                        ai = "generative"
                    elif "digitalCapture" in src and ai is None:
                        ai = "camera"

    return {
        "file": os.path.basename(path),
        "bytes": os.path.getsize(path),
        "camera": camera,
        "gps": gps,
        "credit": credit,
        "c2pa": c2pa,
        "ai": ai,
        "has_any_credit": bool(credit),
    }


# --------------------------------------------------------------------------------------------
# strip
# --------------------------------------------------------------------------------------------

def strip_jpeg(src, dst):
    """Drop every APPn and COM segment. Pixels are untouched: this is a byte-level edit."""
    with open(src, "rb") as f:
        data = f.read()
    if data[:2] != b"\xff\xd8":
        raise ValueError("not a JPEG")
    out = bytearray(b"\xff\xd8")
    i = 2
    removed = 0
    while i < len(data) - 1:
        if data[i] != 0xFF:
            out += data[i:]
            break
        marker = data[i + 1]
        if marker == 0xDA:                       # start of scan: the rest is image data
            out += data[i:]
            break
        if marker in (0xD8, 0xD9) or 0xD0 <= marker <= 0xD7:
            out += data[i:i + 2]
            i += 2
            continue
        if i + 4 > len(data):
            out += data[i:]
            break
        seg_len = struct.unpack(">H", data[i + 2:i + 4])[0]
        if marker in _JPEG_SKIPPABLE:
            removed += seg_len + 2
        else:
            out += data[i:i + 2 + seg_len]
        i += 2 + seg_len
    with open(dst, "wb") as f:
        f.write(out)
    return removed


def strip_png(src, dst):
    """Keep only the chunks a decoder needs. Everything else, including C2PA's caBX, goes."""
    with open(src, "rb") as f:
        data = f.read()
    sig = b"\x89PNG\r\n\x1a\n"
    if data[:8] != sig:
        raise ValueError("not a PNG")
    out = bytearray(sig)
    i = 8
    removed = 0
    while i < len(data):
        if i + 8 > len(data):
            break
        length = struct.unpack(">I", data[i:i + 4])[0]
        ctype = data[i + 4:i + 8]
        total = 12 + length
        if ctype in _PNG_REQUIRED:
            out += data[i:i + total]
        else:
            removed += total
        i += total
        if ctype == b"IEND":
            break
    with open(dst, "wb") as f:
        f.write(out)
    return removed


def strip(src, dst):
    """Returns bytes removed. Raises for a format we cannot do losslessly."""
    ext = os.path.splitext(src)[1].lower()
    if ext in (".jpg", ".jpeg"):
        return strip_jpeg(src, dst)
    if ext == ".png":
        return strip_png(src, dst)
    raise ValueError(f"{ext} is not supported for lossless stripping. Only JPEG and PNG.")


def pixels_identical(a, b):
    """Prove the strip did not touch the image. This is the check that makes the claim honest."""
    try:
        from PIL import Image
    except ImportError:
        return None
    with Image.open(a) as ia, Image.open(b) as ib:
        if ia.size != ib.size or ia.mode != ib.mode:
            return False
        return ia.tobytes() == ib.tobytes()


# --------------------------------------------------------------------------------------------
# credit: write credit
# --------------------------------------------------------------------------------------------

def write_credit(path, *, creator=None, credit=None, copyright_=None, terms=None,
                 rights_url=None, licensor=None, licensor_url=None, contact=None,
                 title=None, description=None, keywords=None, drop_gps=True):
    """Write the IPTC Core / XMP fields that picture desks and Google actually read."""
    require("exiftool", "writing metadata needs it")
    a = ["exiftool", "-overwrite_original", "-q", "-P"]

    def add(*pairs):
        a.extend(pairs)

    if creator:
        add(f"-XMP-dc:Creator={creator}", f"-IPTC:By-line={creator}", f"-EXIF:Artist={creator}")
    if credit:
        add(f"-XMP-photoshop:Credit={credit}", f"-IPTC:Credit={credit}",
            f"-XMP-photoshop:Source={credit}", f"-IPTC:Source={credit}")
    if copyright_:
        add(f"-XMP-dc:Rights={copyright_}", f"-IPTC:CopyrightNotice={copyright_}",
            f"-EXIF:Copyright={copyright_}", "-XMP-xmpRights:Marked=True")
    if terms:
        add(f"-XMP-xmpRights:UsageTerms={terms}")
    if rights_url:
        add(f"-XMP-xmpRights:WebStatement={rights_url}")
    if licensor:
        add(f"-XMP-plus:LicensorName={licensor}")
    if licensor_url:
        add(f"-XMP-plus:LicensorURL={licensor_url}")
    if contact:
        key = "CreatorWorkEmail" if "@" in contact else "CreatorWorkURL"
        add(f"-XMP-iptcCore:{key}={contact}")
    if title:
        add(f"-XMP-dc:Title={title}", f"-IPTC:ObjectName={title}", f"-IPTC:Headline={title}")
    if description:
        add(f"-XMP-dc:Description={description}", f"-IPTC:Caption-Abstract={description}")
    for k in (keywords or []):
        add(f"-XMP-dc:Subject+={k}", f"-IPTC:Keywords+={k}")
    if drop_gps:
        add("-gps:all=")

    if len(a) == 4:
        return False, "nothing to write"
    a.append(path)
    r = _run(a)
    return r.returncode == 0, (r.stderr or "").strip()[:300]


LICENCES = {
    "cc-by": ("CC BY 4.0", "https://creativecommons.org/licenses/by/4.0/"),
    "cc-by-sa": ("CC BY-SA 4.0", "https://creativecommons.org/licenses/by-sa/4.0/"),
    "cc-by-nd": ("CC BY-ND 4.0", "https://creativecommons.org/licenses/by-nd/4.0/"),
    "cc-by-nc": ("CC BY-NC 4.0", "https://creativecommons.org/licenses/by-nc/4.0/"),
    "cc-by-nc-sa": ("CC BY-NC-SA 4.0", "https://creativecommons.org/licenses/by-nc-sa/4.0/"),
    "cc-by-nc-nd": ("CC BY-NC-ND 4.0", "https://creativecommons.org/licenses/by-nc-nd/4.0/"),
    "cc0": ("CC0 1.0", "https://creativecommons.org/publicdomain/zero/1.0/"),
    "arr": ("All rights reserved", None),
}


# --------------------------------------------------------------------------------------------
# stamp: put it in the pixels
# --------------------------------------------------------------------------------------------

CORNERS = ("bottom-right", "bottom-left", "top-right", "top-left")


def stamp(src, dst, *, text, logo=None, corner="bottom-right", scale=0.05, opacity=0.85,
          font=None, quality=94, subtext=None):
    """Composite a visible mark into the pixels.

    THIS RE-ENCODES. It is the only operation here that costs image quality, and the CLI says so.
    Quality defaults to 94, which is visually lossless for a photograph at normal viewing sizes.

    The visible mark is the only layer that survives a screenshot, and on most platforms it is the
    only layer that survives at all."""
    from PIL import Image, ImageDraw, ImageFont

    if corner not in CORNERS:
        raise ValueError(f"corner must be one of {CORNERS}")

    im = Image.open(src).convert("RGBA")
    W, H = im.size
    unit = max(20, int(min(W, H) * scale))
    pad = max(6, int(unit * 0.30))
    text_px = int(unit * 0.50)
    sub_px = int(unit * 0.30)

    try:
        f_main = ImageFont.truetype(font, text_px) if font else ImageFont.load_default(text_px)
        f_sub = ImageFont.truetype(font, sub_px) if font else ImageFont.load_default(sub_px)
    except Exception:
        f_main = ImageFont.load_default()
        f_sub = ImageFont.load_default()

    logo_im = None
    if logo and os.path.exists(logo):
        logo_im = Image.open(logo).convert("RGBA")
        ratio = unit / max(1, logo_im.height)
        logo_im = logo_im.resize((max(1, int(logo_im.width * ratio)), unit), Image.LANCZOS)

    probe = ImageDraw.Draw(Image.new("RGBA", (1, 1)))
    tw = int(probe.textlength(text, font=f_main))
    sw = int(probe.textlength(subtext, font=f_sub)) if subtext else 0
    text_w = max(tw, sw)
    gap = int(unit * 0.30) if logo_im else 0
    lw = logo_im.width if logo_im else 0

    plate_w = pad * 2 + lw + gap + text_w
    plate_h = pad * 2 + unit
    plate = Image.new("RGBA", (plate_w, plate_h), (0, 0, 0, 0))
    d = ImageDraw.Draw(plate)
    d.rounded_rectangle([0, 0, plate_w - 1, plate_h - 1], radius=plate_h // 4,
                        fill=(0, 0, 0, int(255 * 0.42)))
    if logo_im:
        plate.alpha_composite(logo_im, (pad, pad))
    tx = pad + lw + gap
    if subtext:
        d.text((tx, pad + int(unit * 0.06)), text, font=f_main, fill=(255, 255, 255, 255))
        d.text((tx, pad + int(unit * 0.56)), subtext, font=f_sub, fill=(255, 255, 255, 190))
    else:
        d.text((tx, pad + int(unit * 0.26)), text, font=f_main, fill=(255, 255, 255, 255))

    if opacity < 1.0:
        alpha = plate.getchannel("A").point(lambda v: int(v * opacity))
        plate.putalpha(alpha)

    inset = max(8, int(min(W, H) * 0.022))
    x = W - plate_w - inset if "right" in corner else inset
    y = H - plate_h - inset if "bottom" in corner else inset
    im.alpha_composite(plate, (x, y))

    out = im.convert("RGB")
    ext = os.path.splitext(dst)[1].lower()
    if ext == ".png":
        out.save(dst, "PNG")
    else:
        out.save(dst, "JPEG", quality=quality, subsampling=0)
    return dst
