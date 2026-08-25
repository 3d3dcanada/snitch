"""imprint: put your name on your work, and find out what a platform is about to remove.

    imprint inspect  photo.jpg          what is in it (alias: snitch)
    imprint imprint  photo.jpg ...      write credit into it
    imprint stamp    photo.jpg ...      put a visible mark in the pixels
    imprint strip    photo.jpg          take metadata out (alias: no-comment)
    imprint sign     photo.jpg ...      add a C2PA Content Credential
    imprint verify   photo.jpg          check a C2PA Content Credential
    imprint platforms                   what each platform keeps and strips
"""

import argparse
import os
import shutil
import sys

from . import core, survival
from .core import LICENCES

BOLD = "\033[1m"
DIM = "\033[2m"
RED = "\033[31m"
GRN = "\033[32m"
YEL = "\033[33m"
OFF = "\033[0m"


def _c(s, colour):
    return s if not sys.stdout.isatty() else f"{colour}{s}{OFF}"


def _outpath(src, out, suffix):
    if out:
        return out
    stem, ext = os.path.splitext(src)
    return f"{stem}{suffix}{ext}"


# --------------------------------------------------------------------------------------------

def cmd_inspect(a):
    for path in a.files:
        if not os.path.exists(path):
            print(f"  {path}: not found")
            continue
        r = core.inspect(path)
        print(f"\n{_c(r['file'], BOLD)}  {r['bytes']:,} bytes")

        if r["gps"]:
            print(_c("  LOCATION IS IN THIS FILE", RED))
            for k, v in r["gps"].items():
                print(f"    {k.split(':')[-1]:<16} {v}")
            print(_c("    Anyone who downloads this can see where it was taken.", RED))
            print(f"    Remove it:  imprint strip {path}")
        else:
            print(_c("  no location data", GRN))

        if r["camera"]:
            print("  camera")
            for k, v in r["camera"].items():
                print(f"    {k:<16} {v}")

        if r["credit"]:
            print(_c("  credit", GRN))
            for k, v in r["credit"].items():
                if isinstance(v, list):
                    v = ", ".join(str(x) for x in v)
                print(f"    {k:<16} {str(v)[:90]}")
        else:
            print(_c("  NO CREDIT AT ALL", YEL))
            print(f"    Nothing in this file says who made it.")
            print(f"    Fix it:  imprint imprint {path} --creator \"Your Name\"")

        if r["c2pa"]:
            c = r["c2pa"]
            man = (c.get("manifests") or {}).get(c.get("active_manifest"), {})
            sig = man.get("signature_info") or {}
            state = c.get("validation_state", "?")
            print(f"  C2PA Content Credential  [{state}]")
            print(f"    title            {man.get('title', '?')}")
            print(f"    signed by        {sig.get('issuer', '?')}")
            if r["ai"] == "generative":
                print(_c("    THIS SAYS IT WAS MADE BY A GENERATIVE MODEL", YEL))
            elif r["ai"] == "camera":
                print(_c("    asserts a camera made this, not a model", GRN))
        else:
            print("  no C2PA Content Credential")

    print(f"\n{_c('What survives where:', BOLD)}  imprint platforms")
    return 0


def cmd_platforms(a):
    print(f"\n{_c('What each platform does to your metadata', BOLD)}"
          f"   {DIM}verified {survival.VERIFIED}{OFF}\n")
    width = max(len(p) for p in survival.PLATFORMS) + 2
    head = "".join(f"{lbl:<26}" for _, lbl, _ in survival.LAYERS)
    print(f"  {'':<{width}}{head}")
    for name, layers in survival.PLATFORMS.items():
        row = f"  {name:<{width}}"
        for key, _, _ in survival.LAYERS:
            verdict = layers.get(key, (survival.UNKNOWN, ""))[0]
            txt = survival.SYMBOL[verdict]
            colour = {survival.KEEP: GRN, survival.STRIP: RED,
                      survival.PARTIAL: YEL}.get(verdict, DIM)
            row += _c(f"{txt:<26}", colour)
        print(row)

    print(f"\n  {_c(survival.one_line_advice(), BOLD)}\n")
    if a.notes:
        for name, layers in survival.PLATFORMS.items():
            print(f"  {_c(name, BOLD)}")
            for key, lbl, _ in survival.LAYERS:
                v, note = layers.get(key, (survival.UNKNOWN, "not tested"))
                if note:
                    print(f"    {lbl:<26} {note}")
            print()
    else:
        print(f"  {DIM}--notes for the detail on every cell{OFF}")
    if a.check:
        print("\n" + survival.how_to_verify())
    return 0


def cmd_imprint(a):
    lic_name, lic_url = (None, None)
    if a.licence:
        if a.licence not in LICENCES:
            print(f"unknown licence {a.licence!r}. One of: {', '.join(LICENCES)}")
            return 2
        lic_name, lic_url = LICENCES[a.licence]

    terms = a.terms or (f"{lic_name}." if lic_name else None)
    if terms and a.contact:
        terms = f"{terms} Commercial licensing: contact {a.contact}."

    for path in a.files:
        if not os.path.exists(path):
            print(f"  {path}: not found")
            continue
        target = path
        if not a.in_place:
            target = _outpath(path, a.out if len(a.files) == 1 else None, "-imprint")
            shutil.copy2(path, target)

        if a.stamp_text or a.logo:
            core.stamp(target, target, text=a.stamp_text or (a.creator or ""),
                       subtext=a.stamp_sub, logo=a.logo, corner=a.corner,
                       scale=a.scale, opacity=a.opacity, font=a.font, quality=a.quality)

        ok, err = core.write_credit(
            target, creator=a.creator, credit=a.credit, copyright_=a.copyright,
            terms=terms, rights_url=a.rights_url or lic_url, licensor=a.credit,
            licensor_url=a.url, contact=a.contact, title=a.title,
            description=a.description, keywords=a.keyword, drop_gps=not a.keep_gps)
        mark = _c("ok", GRN) if ok else _c("FAILED " + err, RED)
        stamped = " + visible stamp" if (a.stamp_text or a.logo) else ""
        print(f"  {os.path.basename(target)}  credit written{stamped}  {mark}")
        if a.stamp_text or a.logo:
            print(f"    {DIM}stamping re-encoded the image at quality {a.quality}{OFF}")
    return 0


def cmd_stamp(a):
    for path in a.files:
        if not os.path.exists(path):
            print(f"  {path}: not found")
            continue
        out = _outpath(path, a.out if len(a.files) == 1 else None, "-stamped")
        core.stamp(path, out, text=a.text, subtext=a.sub, logo=a.logo, corner=a.corner,
                   scale=a.scale, opacity=a.opacity, font=a.font, quality=a.quality)
        print(f"  {os.path.basename(out)}  stamped  "
              f"{DIM}re-encoded at quality {a.quality}{OFF}")
    return 0


def cmd_strip(a):
    for path in a.files:
        if not os.path.exists(path):
            print(f"  {path}: not found")
            continue
        out = path if a.in_place else _outpath(path, a.out if len(a.files) == 1 else None,
                                               "-clean")
        try:
            if a.in_place:
                tmp = path + ".tmp"
                removed = core.strip(path, tmp)
                same = core.pixels_identical(path, tmp)
                shutil.move(tmp, path)
            else:
                removed = core.strip(path, out)
                same = core.pixels_identical(path, out)
        except ValueError as e:
            print(f"  {os.path.basename(path)}: {e}")
            continue
        proof = ""
        if same is True:
            proof = _c("  pixels byte-identical", GRN)
        elif same is False:
            proof = _c("  PIXELS CHANGED, this is a bug, please report it", RED)
        print(f"  {os.path.basename(out)}  removed {removed:,} bytes of metadata{proof}")

    print(f"\n  {_c('What this does not do:', BOLD)} in-pixel watermarks such as Google SynthID")
    print("  are part of the image itself. They survive re-encoding, cropping and resizing by")
    print("  design, and no tool removes them, including this one.")
    return 0


def cmd_sign(a):
    from . import sign as signer
    return signer.run(a)


def cmd_verify(a):
    for path in a.files:
        c = core.read_c2pa(path)
        if not c:
            print(f"  {os.path.basename(path)}: no Content Credential")
            continue
        man = (c.get("manifests") or {}).get(c.get("active_manifest"), {})
        sig = man.get("signature_info") or {}
        state = c.get("validation_state", "?")
        colour = GRN if state == "Valid" else YEL
        print(f"  {os.path.basename(path)}  {_c(state, colour)}  "
              f"signed by {sig.get('issuer', '?')}  {man.get('title', '')}")
    return 0


# --------------------------------------------------------------------------------------------

def build_parser():
    p = argparse.ArgumentParser(
        prog="imprint",
        description="Put your name on your work, and find out what a platform will remove.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__)
    sub = p.add_subparsers(dest="cmd", required=True)

    def files(sp):
        sp.add_argument("files", nargs="+", help="image files")

    ins = sub.add_parser("inspect", aliases=["snitch"],
                         help="what is in this image, including location and AI provenance")
    files(ins)
    ins.set_defaults(fn=cmd_inspect)

    plt = sub.add_parser("platforms", help="what each platform keeps and strips")
    plt.add_argument("--notes", action="store_true", help="the detail behind every cell")
    plt.add_argument("--check", action="store_true", help="how to verify a row yourself")
    plt.set_defaults(fn=cmd_platforms)

    imp = sub.add_parser("imprint", help="write credit into an image")
    files(imp)
    imp.add_argument("--creator", help="the person who made it")
    imp.add_argument("--credit", help="the studio or organisation to credit")
    imp.add_argument("--copyright", help="copyright notice")
    imp.add_argument("--licence", "--license", dest="licence",
                     help=f"one of: {', '.join(LICENCES)}")
    imp.add_argument("--terms", help="usage terms, overrides --licence text")
    imp.add_argument("--rights-url", help="URL of the rights statement")
    imp.add_argument("--url", help="licensor URL")
    imp.add_argument("--contact", help="email or URL for licensing enquiries")
    imp.add_argument("--title")
    imp.add_argument("--description")
    imp.add_argument("--keyword", action="append", default=[], help="repeatable")
    imp.add_argument("--keep-gps", action="store_true",
                     help="keep location data. Off by default, because it doxxes people")
    imp.add_argument("--stamp-text", help="also burn this text into the pixels")
    imp.add_argument("--stamp-sub", help="second line under the stamp text")
    imp.add_argument("--logo", help="PNG logo for the stamp")
    imp.add_argument("--corner", default="bottom-right", choices=core.CORNERS)
    imp.add_argument("--scale", type=float, default=0.05)
    imp.add_argument("--opacity", type=float, default=0.85)
    imp.add_argument("--font", help="path to a .ttf")
    imp.add_argument("--quality", type=int, default=94)
    imp.add_argument("--in-place", action="store_true")
    imp.add_argument("-o", "--out")
    imp.set_defaults(fn=cmd_imprint)

    stp = sub.add_parser("stamp", help="put a visible mark in the pixels")
    files(stp)
    stp.add_argument("--text", required=True)
    stp.add_argument("--sub", help="second line")
    stp.add_argument("--logo")
    stp.add_argument("--corner", default="bottom-right", choices=core.CORNERS)
    stp.add_argument("--scale", type=float, default=0.05)
    stp.add_argument("--opacity", type=float, default=0.85)
    stp.add_argument("--font")
    stp.add_argument("--quality", type=int, default=94)
    stp.add_argument("-o", "--out")
    stp.set_defaults(fn=cmd_stamp)

    srp = sub.add_parser("strip", aliases=["no-comment"],
                         help="remove metadata, losslessly")
    files(srp)
    srp.add_argument("--in-place", action="store_true")
    srp.add_argument("-o", "--out")
    srp.set_defaults(fn=cmd_strip)

    sgn = sub.add_parser("sign", help="add a C2PA Content Credential")
    files(sgn)
    sgn.add_argument("--creator")
    sgn.add_argument("--org")
    sgn.add_argument("--url")
    sgn.add_argument("--contact")
    sgn.add_argument("--licence", "--license", dest="licence")
    sgn.add_argument("--title")
    sgn.add_argument("--description")
    sgn.add_argument("--key", help="PEM private key. Generated on first run if absent")
    sgn.add_argument("--cert", help="PEM certificate")
    sgn.add_argument("--generated", action="store_true",
                     help="declare this was made by a generative model, not a camera")
    sgn.set_defaults(fn=cmd_sign)

    vfy = sub.add_parser("verify", help="check a C2PA Content Credential")
    files(vfy)
    vfy.set_defaults(fn=cmd_verify)
    return p


def main(argv=None):
    args = build_parser().parse_args(argv)
    try:
        return args.fn(args)
    except core.ToolMissing as e:
        print(f"\n{e}\n")
        return 3
    except KeyboardInterrupt:
        return 130


if __name__ == "__main__":
    sys.exit(main())
