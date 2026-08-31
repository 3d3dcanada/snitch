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

import contextlib
import hashlib
import json
import os
import shutil
import struct
import subprocess
import tempfile
import zlib

# JPEG application markers usually carry metadata, but three are required for faithful interchange:
# JFIF/JFXX identifies a JFIF stream, ICC_PROFILE controls colour, and Adobe APP14 describes the
# colour transform used by many CMYK JPEGs. EXIF is replaced with an orientation-only segment below.
_JPEG_METADATA = set(range(0xE0, 0xF0)) | {0xFE}          # APP0..APP15 and COM
_PNG_DISPLAY_CHUNKS = {
    b"PLTE", b"tRNS", b"gAMA", b"cHRM", b"sRGB", b"iCCP", b"cICP", b"mDCv", b"cLLi",
    b"sBIT", b"pHYs", b"bKGD", b"acTL", b"fcTL", b"fdAT",
}

# The three PNG text chunks. tEXt is a Latin-1 keyword and text separated by a NUL. zTXt adds a
# compression method byte and deflates the text. iTXt is UTF-8 and carries a compression flag, a
# language tag and a translated keyword before the text. Generators write into all three, so a
# reader that only understands tEXt misses most of what is actually in the file.
_PNG_TEXT_CHUNKS = {b"tEXt", b"zTXt", b"iTXt"}

# Keywords that image generators write. Matching the keyword rather than the text is deliberate:
# a photograph whose caption happens to mention Stable Diffusion is still a photograph.
_GENERATOR_KEYWORDS = {
    "parameters",        # Stable Diffusion WebUI: AUTOMATIC1111, Forge
    "prompt",            # ComfyUI
    "workflow",          # ComfyUI
    "dream",             # early Stable Diffusion
    "sd-metadata",       # InvokeAI
    "invokeai_metadata",
    "invokeai_graph",
    "aigenerated",
    "generation_data",
}


class ToolMissing(RuntimeError):
    pass


def _run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, check=False, **kw)


def have(tool):
    return shutil.which(tool) is not None


def require(tool, why):
    if not have(tool):
        debian_package = "libimage-exiftool-perl" if tool == "exiftool" else tool
        raise ToolMissing(
            f"{tool} is not installed, and {why}.\n"
            f"  Debian/Ubuntu:  sudo apt install {debian_package}\n"
            f"  macOS:          brew install {tool}\n"
            f"  Windows:        choco install {tool}"
        )


# --------------------------------------------------------------------------------------------
# inspect
# --------------------------------------------------------------------------------------------

def read_metadata(path):
    """Everything exiftool can see, as a dict."""
    path = os.path.abspath(os.fspath(path))
    if not os.path.isfile(path):
        raise ValueError("not a regular file")
    require("exiftool", "reading metadata needs it")
    r = _run(["exiftool", "-j", "-G", "-n", "-a", "-u", path])
    if not r.stdout.strip():
        error = r.stderr.strip()[:300]
        raise ValueError(error or "ExifTool could not read this file")
    try:
        reports = json.loads(r.stdout)
        metadata = reports[0]
    except (IndexError, TypeError, json.JSONDecodeError) as exc:
        raise ValueError(f"invalid ExifTool JSON: {exc}") from exc
    if metadata.get("ExifTool:Error"):
        raise ValueError(str(metadata["ExifTool:Error"]))
    if r.returncode:
        raise ValueError(r.stderr.strip()[:300] or f"ExifTool exited {r.returncode}")
    mime = metadata.get("File:MIMEType", "")
    if not str(mime).startswith("image/"):
        kind = metadata.get("File:FileType", "unknown")
        raise ValueError(f"not an image (ExifTool identified {kind})")
    return metadata


def _png_text_payload(ctype, payload):
    """Decode one text chunk into (keyword, text), or None if it is malformed."""
    keyword, sep, rest = payload.partition(b"\x00")
    if not sep:
        return None
    name = keyword.decode("latin-1", "replace")
    if ctype == b"tEXt":
        return name, rest.decode("latin-1", "replace")
    if ctype == b"zTXt":
        if not rest or rest[0] != 0:              # 0 is the only defined compression method
            return None
        try:
            return name, zlib.decompress(rest[1:]).decode("latin-1", "replace")
        except zlib.error:
            return None
    # iTXt: compression flag, compression method, language tag, translated keyword, then text.
    if len(rest) < 2:
        return None
    compressed, method = rest[0], rest[1]
    _language, sep, rest = rest[2:].partition(b"\x00")
    if not sep:
        return None
    _translated, sep, text = rest.partition(b"\x00")
    if not sep:
        return None
    if compressed:
        if method != 0:
            return None
        try:
            text = zlib.decompress(text)
        except zlib.error:
            return None
    return name, text.decode("utf-8", "replace")


def read_png_text(path):
    """Every tEXt, zTXt and iTXt chunk in a PNG, decoded, in file order.

    ExifTool folds these into its PNG group beside IHDR fields, with nothing to distinguish a
    generator's prompt from an image dimension. Reading the container directly keeps the chunk
    type, which is the part worth telling someone about.

    Reporting is not surgery: this walker stops at the first inconsistency and returns what it
    read, so a damaged file still reports what could be recovered. strip_png stays strict, and
    is the one that refuses.
    """
    path = os.path.abspath(os.fspath(path))
    with open(path, "rb") as f:
        data = f.read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        return []
    found = []
    i = 8
    while i + 12 <= len(data):
        length = struct.unpack(">I", data[i:i + 4])[0]
        ctype = data[i + 4:i + 8]
        total = 12 + length
        if i + total > len(data):
            break
        if ctype in _PNG_TEXT_CHUNKS:
            decoded = _png_text_payload(ctype, data[i + 8:i + 8 + length])
            if decoded:
                found.append({"chunk": ctype.decode("ascii"),
                              "keyword": decoded[0],
                              "text": decoded[1]})
        i += total
        if ctype == b"IEND":
            break
    return found


def read_c2pa(path, c2patool=None):
    """The C2PA manifest, or None. Never raises: absence is the common case."""
    _, data, _ = read_c2pa_report(path, c2patool)
    return data


def resolve_c2patool(tool=None):
    candidate = tool or "c2patool"
    resolved = shutil.which(candidate)
    if resolved:
        return resolved
    if tool:
        return None
    cargo_path = os.path.expanduser("~/.cargo/bin/c2patool")
    return cargo_path if os.path.isfile(cargo_path) and os.access(cargo_path, os.X_OK) else None


def read_c2pa_report(path, c2patool=None):
    """Return (status, manifest, error) without conflating no tool, no claim, and failure."""
    path = os.path.abspath(os.fspath(path))
    tool = resolve_c2patool(c2patool)
    if not tool:
        return "unavailable", None, "c2patool is not installed"
    r = _run([tool, path])
    if r.returncode:
        error = (r.stderr or r.stdout).strip()
        if "No claim found" in error:
            return "absent", None, ""
        return "error", None, error[:300] or f"c2patool exited {r.returncode}"
    if not r.stdout.strip():
        return "error", None, "c2patool returned no report"
    try:
        return "present", json.loads(r.stdout), ""
    except (TypeError, json.JSONDecodeError) as exc:
        return "error", None, f"invalid c2patool JSON: {exc}"


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
    c2pa_status, c2pa, c2pa_error = read_c2pa_report(path, c2patool)
    if c2pa_status == "unavailable" and meta.get("JUMBF:JUMDLabel") == "c2pa":
        c2pa_status = "detected-unverified"

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

    png_text = read_png_text(path)
    generator_chunks = [c for c in png_text if c["keyword"].lower() in _GENERATOR_KEYWORDS]

    # A C2PA claim is cryptographically bound to the asset. A text chunk is plain text that anyone
    # can write and that survives being copied around. Both are worth reporting, they are not worth
    # the same, and the caller is told which one answered so a chunk can never pass for a credential.
    ai = None
    ai_source = None
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
        if ai is not None:
            ai_source = "c2pa"
    if ai is None and generator_chunks:
        ai = "generative"
        ai_source = "png-text-chunk"

    return {
        "file": os.path.basename(path),
        "path": os.path.abspath(path),
        "bytes": os.path.getsize(path),
        "mime_type": meta.get("File:MIMEType"),
        "camera": camera,
        "gps": gps,
        "credit": credit,
        "png_text": png_text,
        "generator_keywords": sorted({c["keyword"] for c in generator_chunks}),
        "c2pa": c2pa,
        "c2pa_status": c2pa_status,
        "c2pa_error": c2pa_error,
        "ai": ai,
        "ai_source": ai_source,
        "has_any_credit": bool(credit),
    }


# --------------------------------------------------------------------------------------------
# strip
# --------------------------------------------------------------------------------------------

def strip_jpeg(src, dst):
    """Drop private/application metadata while retaining display-critical JPEG segments."""
    with open(src, "rb") as f:
        data = f.read()
    if data[:2] != b"\xff\xd8":
        raise ValueError("not a JPEG")

    orientation_segment = _jpeg_orientation_segment(src)
    out = bytearray(b"\xff\xd8")
    i = 2
    inserted_orientation = False
    saw_scan = False
    while i < len(data) - 1:
        if data[i] != 0xFF:
            raise ValueError("malformed JPEG marker stream")
        marker_start = i
        while i < len(data) and data[i] == 0xFF:
            i += 1
        if i >= len(data):
            raise ValueError("truncated JPEG marker")
        marker = data[i]
        i += 1
        if marker == 0x00:
            raise ValueError("unexpected stuffed byte before JPEG scan")
        if marker in (0x01, 0xD8, 0xD9) or 0xD0 <= marker <= 0xD7:
            out += data[marker_start:i]
            continue
        if i + 2 > len(data):
            raise ValueError("truncated JPEG segment length")
        seg_len = struct.unpack(">H", data[i:i + 2])[0]
        if seg_len < 2:
            raise ValueError("invalid JPEG segment length")
        segment_end = i + seg_len
        if segment_end > len(data):
            raise ValueError("truncated JPEG segment")

        if marker == 0xDA:                       # start of scan: the rest is image data
            if orientation_segment and not inserted_orientation:
                out += orientation_segment
            if data.find(b"\xff\xd9", segment_end) < 0:
                raise ValueError("JPEG scan has no end marker")
            out += data[marker_start:]
            saw_scan = True
            break

        payload = data[i + 2:segment_end]
        keep = marker not in _JPEG_METADATA or _keep_jpeg_application_segment(marker, payload)
        if orientation_segment and not inserted_orientation and marker != 0xE0:
            out += orientation_segment
            inserted_orientation = True
        if keep:
            out += data[marker_start:segment_end]
        i = segment_end
    if not saw_scan:
        raise ValueError("JPEG has no image scan")
    with open(dst, "wb") as f:
        f.write(out)
    return len(data) - len(out)


def _keep_jpeg_application_segment(marker, payload):
    return (
        (marker == 0xE0 and payload.startswith((b"JFIF\x00", b"JFXX\x00")))
        or (marker == 0xE2 and payload.startswith(b"ICC_PROFILE\x00"))
        or (marker == 0xEE and payload.startswith(b"Adobe"))
    )


def _orientation_value(path):
    try:
        from PIL import Image
        with Image.open(path) as image:
            value = image.getexif().get(274)
    except (OSError, SyntaxError, ValueError):
        return None
    return value if value in range(2, 9) else None


def _orientation_payload(path):
    orientation = _orientation_value(path)
    if orientation is None:
        return None
    from PIL import Image
    exif = Image.Exif()
    exif[274] = orientation
    return exif.tobytes()


def _jpeg_orientation_segment(path):
    payload = _orientation_payload(path)
    if payload is None:
        return None
    if len(payload) + 2 > 0xFFFF:
        raise ValueError("orientation EXIF segment is too large")
    return b"\xff\xe1" + struct.pack(">H", len(payload) + 2) + payload


def strip_png(src, dst):
    """Keep image/display chunks and drop private metadata, including C2PA's caBX chunk."""
    with open(src, "rb") as f:
        data = f.read()
    sig = b"\x89PNG\r\n\x1a\n"
    if data[:8] != sig:
        raise ValueError("not a PNG")
    out = bytearray(sig)
    i = 8
    saw_header = False
    saw_end = False
    inserted_orientation = False
    orientation_chunk = _png_orientation_chunk(src)
    while i < len(data):
        if i + 12 > len(data):
            raise ValueError("truncated PNG chunk")
        length = struct.unpack(">I", data[i:i + 4])[0]
        ctype = data[i + 4:i + 8]
        total = 12 + length
        if i + total > len(data):
            raise ValueError("truncated PNG chunk payload")
        payload = data[i + 8:i + 8 + length]
        expected_crc = struct.unpack(">I", data[i + 8 + length:i + total])[0]
        actual_crc = zlib.crc32(ctype + payload) & 0xFFFFFFFF
        if expected_crc != actual_crc:
            raise ValueError(f"bad PNG CRC in {ctype.decode('ascii', 'replace')} chunk")
        if not saw_header and ctype != b"IHDR":
            raise ValueError("PNG does not start with IHDR")
        if ctype == b"IHDR":
            if saw_header:
                raise ValueError("PNG has more than one IHDR chunk")
            saw_header = True
        if ctype == b"IDAT" and orientation_chunk and not inserted_orientation:
            out += orientation_chunk
            inserted_orientation = True
        is_critical = 65 <= ctype[0] <= 90
        if is_critical or ctype in _PNG_DISPLAY_CHUNKS:
            out += data[i:i + total]
        i += total
        if ctype == b"IEND":
            saw_end = True
            break
    if not saw_header or not saw_end:
        raise ValueError("PNG is missing a required terminal chunk")
    if i != len(data):
        raise ValueError("PNG has data after IEND")
    with open(dst, "wb") as f:
        f.write(out)
    return len(data) - len(out)


def _png_chunk(ctype, payload):
    crc = zlib.crc32(ctype + payload) & 0xFFFFFFFF
    return struct.pack(">I", len(payload)) + ctype + payload + struct.pack(">I", crc)


def _png_orientation_chunk(path):
    payload = _orientation_payload(path)
    if payload is None:
        return None
    if payload.startswith(b"Exif\x00\x00"):
        payload = payload[6:]
    return _png_chunk(b"eXIf", payload)


def strip(src, dst):
    """Returns bytes removed. Raises for a format we cannot do losslessly."""
    ext = os.path.splitext(src)[1].lower()
    if ext in (".jpg", ".jpeg"):
        return strip_jpeg(src, dst)
    if ext == ".png":
        return strip_png(src, dst)
    raise ValueError(f"{ext} is not supported for lossless stripping. Only JPEG and PNG.")


# ----------------------------------------------------------------------------------------------
# safe output. Shared by every caller so nobody reimplements the guards more weakly.
# ----------------------------------------------------------------------------------------------

def outpath(src, out, suffix):
    if out:
        return out
    stem, ext = os.path.splitext(src)
    return f"{stem}{suffix}{ext}"


def same_file(a, b):
    try:
        return os.path.samefile(a, b)
    except (FileNotFoundError, OSError):
        return os.path.normcase(os.path.abspath(a)) == os.path.normcase(os.path.abspath(b))


def temporary_sibling(path, keep_extension=False):
    directory = os.path.dirname(os.path.abspath(path))
    basename = os.path.basename(path)
    stem, extension = os.path.splitext(basename)
    prefix = f".{stem if keep_extension else basename}.snitch-"
    suffix = extension if keep_extension else ".tmp"
    fd, temporary = tempfile.mkstemp(prefix=prefix, suffix=suffix, dir=directory)
    os.close(fd)
    return temporary


def strip_atomic(source, target):
    """Strip into a sibling temp file, prove the pixels survived, then move it into place.

    Nothing partial ever appears at the target: either the pixel check passes and the file is
    replaced in one operation, or the temporary is removed and the target is untouched.
    """
    temporary = temporary_sibling(target)
    try:
        removed = strip(source, temporary)
        same = pixels_identical(source, temporary)
        if same is not True:
            reason = "pixel comparison unavailable" if same is None else "pixels changed"
            raise ValueError(f"refusing to write output: {reason}")
        shutil.copystat(source, temporary, follow_symlinks=True)
        os.replace(temporary, target)
        return removed, same
    finally:
        with contextlib.suppress(FileNotFoundError):
            os.unlink(temporary)


# A hash mismatch is the validator saying the bytes it is looking at are not the bytes that were
# signed. That is a different fact from an untrusted signer, and the two must never be merged:
# one means the image changed, the other means we cannot say who signed it.
ALTERED_CODES = {
    "assertion.dataHash.mismatch",
    "assertion.bmffHash.mismatch",
    "assertion.boxesHash.mismatch",
}


def _status_codes(c2pa_report):
    return {status.get("code")
            for status in ((c2pa_report or {}).get("validation_status") or [])
            if isinstance(status, dict)}


def asset_altered(c2pa_report):
    """True when the validator says these pixels are not the ones that were signed."""
    return bool(_status_codes(c2pa_report) & ALTERED_CODES)


def identity_untrusted(c2pa_report):
    """True when the signing certificate is not on the validator's trust list."""
    return "signingCredential.untrusted" in _status_codes(c2pa_report)


def pixels_identical(a, b):
    """Prove the strip did not touch the image. This is the check that makes the claim honest."""
    try:
        from PIL import Image
    except ImportError:
        return None
    def pixel_digest(path):
        digest = hashlib.sha256()
        with Image.open(path) as image:
            digest.update(repr((image.size, image.mode, getattr(image, "n_frames", 1))).encode())
            for frame in range(getattr(image, "n_frames", 1)):
                image.seek(frame)
                digest.update(repr((image.size, image.mode)).encode())
                rows = max(1, (1024 * 1024) // max(1, image.width * len(image.getbands())))
                for top in range(0, image.height, rows):
                    digest.update(image.crop((0, top, image.width,
                                              min(image.height, top + rows))).tobytes())
        return digest.digest()

    return pixel_digest(a) == pixel_digest(b)


# --------------------------------------------------------------------------------------------
# credit: write credit
# --------------------------------------------------------------------------------------------

def write_credit(path, *, creator=None, credit=None, copyright_=None, terms=None,
                 rights_url=None, licensor=None, licensor_url=None, contact=None,
                 title=None, description=None, keywords=None, drop_gps=True):
    """Write the IPTC Core / XMP fields that picture desks and Google actually read."""
    path = os.path.abspath(os.fspath(path))
    require("exiftool", "writing metadata needs it")
    a = ["exiftool", "-overwrite_original", "-q", "-P"]

    def add(*pairs):
        a.extend(pairs)

    writes_iptc = any((creator, credit, copyright_, title, description, keywords))
    if writes_iptc:
        add("-IPTC:CodedCharacterSet=UTF8")
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
STAMP_EXTENSIONS = (".jpg", ".jpeg", ".png")


def stamp(src, dst, *, text, logo=None, corner="bottom-right", scale=0.05, opacity=0.85,
          font=None, quality=94, subtext=None):
    """Composite a visible mark into the pixels.

    THIS RE-ENCODES. It is the only operation here that costs image quality, and the CLI says so.
    Quality defaults to 94, which is visually lossless for a photograph at normal viewing sizes.

    A visible mark survives a screenshot because it is part of the pixels, but platforms may crop,
    resize, or soften it."""
    from PIL import Image, ImageDraw, ImageFont

    # THE RE-ENCODE MUST CARRY EXIF AND ICC ACROSS, and this is not a nicety. Pillow writes a
    # fresh file, so without this the stamp silently destroys the camera block, the colour
    # profile and the GPS. That made `credit --stamp --keep-gps` a lie: it kept nothing, because
    # the stamp had already thrown the EXIF away before the metadata step ran. Found by using
    # the tool rather than by testing its parts.
    if corner not in CORNERS:
        raise ValueError(f"corner must be one of {CORNERS}")
    if not 0 < scale <= 1:
        raise ValueError("scale must be greater than 0 and at most 1")
    if not 0 < opacity <= 1:
        raise ValueError("opacity must be greater than 0 and at most 1")
    if not 1 <= quality <= 100:
        raise ValueError("quality must be between 1 and 100")
    ext = os.path.splitext(dst)[1].lower()
    if ext not in STAMP_EXTENSIONS:
        raise ValueError("visible stamping supports JPEG and PNG only")
    if not text and not logo:
        raise ValueError("a visible stamp needs text or a logo")
    if logo and not os.path.isfile(logo):
        raise ValueError(f"logo not found: {logo}")

    _carry = {}
    with Image.open(src) as source:
        source_has_alpha = "A" in source.getbands() or "transparency" in source.info
        # Grab these BEFORE convert(), which does not carry them onto the new image.
        if source.info.get("exif"):
            _carry["exif"] = source.info["exif"]
        if source.info.get("icc_profile"):
            _carry["icc_profile"] = source.info["icc_profile"]
        im = source.convert("RGBA")
    W, H = im.size
    unit = max(20, int(min(W, H) * scale))
    pad = max(6, int(unit * 0.30))
    text_px = int(unit * 0.50)
    sub_px = int(unit * 0.30)

    f_main = ImageFont.truetype(font, text_px) if font else ImageFont.load_default(size=text_px)
    f_sub = ImageFont.truetype(font, sub_px) if font else ImageFont.load_default(size=sub_px)

    logo_im = None
    if logo:
        with Image.open(logo) as source_logo:
            logo_im = source_logo.convert("RGBA")
        ratio = unit / max(1, logo_im.height)
        logo_im = logo_im.resize(
            (max(1, int(logo_im.width * ratio)), unit), Image.Resampling.LANCZOS
        )

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
    available_w = max(1, W - 2 * inset)
    available_h = max(1, H - 2 * inset)
    fit = min(1.0, available_w / plate.width, available_h / plate.height)
    if fit < 1.0:
        plate = plate.resize(
            (max(1, int(plate.width * fit)), max(1, int(plate.height * fit))),
            Image.Resampling.LANCZOS,
        )
        plate_w, plate_h = plate.size
    x = W - plate_w - inset if "right" in corner else inset
    y = H - plate_h - inset if "bottom" in corner else inset
    im.alpha_composite(plate, (x, y))

    if ext == ".png":
        out = im if source_has_alpha else im.convert("RGB")
        out.save(dst, "PNG", **{k: v for k, v in _carry.items() if k == "icc_profile"})
    else:
        out = im.convert("RGB")
        out.save(dst, "JPEG", quality=quality, subsampling=0, **_carry)
    return dst
