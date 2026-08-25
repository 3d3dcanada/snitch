"""Three tools, three commands. You type the name of the thing you want.

    snitch      photo.jpg              what is this file telling people about you
    no-comment  photo.jpg              make it stop
    credit      photo.jpg --creator    put your name on it, so it stays

There is deliberately no umbrella command with these hidden underneath as subcommands. Each tool
is the thing it is called, because the name IS the interface: you remember "snitch" long after you
have forgotten which flag of which subcommand read GPS.
"""

import argparse
import os
import shutil
import sys
import tempfile

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


def _output_paths(files, out, suffix):
    if out and os.path.isdir(out):
        targets = [_outpath(os.path.join(out, os.path.basename(path)), None, suffix)
                   for path in files]
    elif out and len(files) > 1:
        raise ValueError("--out must be an existing directory when processing multiple files")
    else:
        targets = [_outpath(path, out, suffix) for path in files]
    canonical = [os.path.normcase(os.path.abspath(path)) for path in targets]
    if len(canonical) != len(set(canonical)):
        raise ValueError("multiple inputs would write the same output filename")
    return targets


def _same_file(a, b):
    try:
        return os.path.samefile(a, b)
    except (FileNotFoundError, OSError):
        return os.path.normcase(os.path.abspath(a)) == os.path.normcase(os.path.abspath(b))


def _temporary_sibling(path):
    directory = os.path.dirname(os.path.abspath(path))
    prefix = f".{os.path.basename(path)}.snitch-"
    fd, temporary = tempfile.mkstemp(prefix=prefix, suffix=".tmp", dir=directory)
    os.close(fd)
    return temporary


def _strip_atomic(source, target):
    temporary = _temporary_sibling(target)
    try:
        removed = core.strip(source, temporary)
        same = core.pixels_identical(source, temporary)
        if same is not True:
            reason = "pixel comparison unavailable" if same is None else "pixels changed"
            raise ValueError(f"refusing to write output: {reason}")
        shutil.copystat(source, temporary, follow_symlinks=True)
        os.replace(temporary, target)
        return removed, same
    finally:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass


def _files(sp):
    sp.add_argument("files", nargs="*", help="image files")


# ==============================================================================================
# snitch: what is this file telling people
# ==============================================================================================

def print_platforms(notes=False, check=False):
    print(f"\n{_c('What each platform does to your metadata', BOLD)}"
          f"   {DIM}verified {survival.VERIFIED}{OFF}\n")
    width = max(len(p) for p in survival.PLATFORMS) + 2
    head = "".join(f"{lbl:<26}" for _, lbl, _ in survival.LAYERS)
    print(f"  {'':<{width}}{head}")
    for name, layers in survival.PLATFORMS.items():
        row = f"  {name:<{width}}"
        for key, _, _ in survival.LAYERS:
            verdict = layers.get(key, (survival.UNKNOWN, ""))[0]
            colour = {survival.KEEP: GRN, survival.STRIP: RED,
                      survival.PARTIAL: YEL}.get(verdict, DIM)
            row += _c(f"{survival.SYMBOL[verdict]:<26}", colour)
        print(row)
    print(f"\n  {_c(survival.one_line_advice(), BOLD)}\n")
    if notes:
        for name, layers in survival.PLATFORMS.items():
            print(f"  {_c(name, BOLD)}")
            for key, lbl, _ in survival.LAYERS:
                _, note = layers.get(key, (survival.UNKNOWN, "not tested"))
                if note:
                    print(f"    {lbl:<26} {note}")
            print()
    else:
        print(f"  {DIM}--notes for the detail on every cell{OFF}")
    if check:
        print("\n" + survival.how_to_verify())


def snitch_main(argv=None):
    p = argparse.ArgumentParser(
        prog="snitch",
        description="Your photo is telling on you. This says what it is telling.",
        epilog="Then:  no-comment FILE   to make it stop\n"
               "       credit FILE       to put your name on it instead",
        formatter_class=argparse.RawDescriptionHelpFormatter)
    _files(p)
    p.add_argument("--platforms", action="store_true",
                   help="what each platform keeps and strips on upload")
    p.add_argument("--notes", action="store_true", help="detail behind every cell")
    p.add_argument("--check", action="store_true", help="how to verify a row yourself")
    a = p.parse_args(argv)

    if a.platforms or not a.files:
        print_platforms(a.notes, a.check)
        if not a.files:
            return 0

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
            print(f"    Make it stop:  no-comment {path}")
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
            print("    Nothing in this file says who made it.")
            print(f"    Fix it:  credit {path} --creator \"Your Name\"")

        if r["c2pa"]:
            c = r["c2pa"]
            man = (c.get("manifests") or {}).get(c.get("active_manifest"), {})
            sig = man.get("signature_info") or {}
            print(f"  C2PA Content Credential  [{c.get('validation_state', '?')}]")
            print(f"    title            {man.get('title', '?')}")
            print(f"    signed by        {sig.get('issuer', '?')}")
            if r["ai"] == "generative":
                print(_c("    THIS SAYS IT WAS MADE BY A GENERATIVE MODEL", YEL))
            elif r["ai"] == "camera":
                print(_c("    asserts a camera made this, not a model", GRN))
        else:
            print("  no C2PA Content Credential")

    print(f"\n  {DIM}snitch --platforms   what survives an upload and what does not{OFF}")
    return 0


# ==============================================================================================
# no-comment: make it stop
# ==============================================================================================

def nocomment_main(argv=None):
    p = argparse.ArgumentParser(
        prog="no-comment",
        description="Strip metadata out of an image. Losslessly: the pixels do not change.",
        epilog="What this does NOT do: in-pixel watermarks such as Google SynthID are part of\n"
               "the image itself. They survive re-encoding, cropping and resizing by design,\n"
               "and no tool removes them, including this one.",
        formatter_class=argparse.RawDescriptionHelpFormatter)
    _files(p)
    destination = p.add_mutually_exclusive_group()
    destination.add_argument("--in-place", action="store_true",
                             help="atomically replace each input file")
    destination.add_argument("-o", "--out",
                             help="output file, or an existing directory for multiple inputs")
    p.add_argument("--force", action="store_true", help="replace an existing output file")
    a = p.parse_args(argv)
    if not a.files:
        p.print_help()
        return 2

    try:
        outputs = _output_paths(a.files, a.out, "-clean")
    except ValueError as e:
        p.error(str(e))

    failed = False
    for path, planned_output in zip(a.files, outputs):
        if not os.path.exists(path):
            print(f"  {path}: not found", file=sys.stderr)
            failed = True
            continue
        if a.in_place and os.path.islink(path):
            print(f"  {path}: refusing in-place replacement of a symlink", file=sys.stderr)
            failed = True
            continue
        out = path if a.in_place else planned_output
        if not a.in_place and _same_file(path, out):
            print(f"  {path}: output is the input; use --in-place", file=sys.stderr)
            failed = True
            continue
        if not a.in_place and os.path.lexists(out) and not a.force:
            print(f"  {out}: output exists; pass --force to replace it", file=sys.stderr)
            failed = True
            continue
        try:
            removed, same = _strip_atomic(path, out)
        except (OSError, ValueError) as e:
            print(f"  {os.path.basename(path)}: {e}", file=sys.stderr)
            failed = True
            continue
        proof = ""
        if same is True:
            proof = _c("  pixels byte-identical", GRN)
        elif same is False:
            proof = _c("  PIXELS CHANGED, this is a bug, please report it", RED)
        print(f"  {os.path.basename(out)}  removed {removed:,} bytes of metadata{proof}")

    print(f"\n  {_c('In-pixel watermarks are not touched by this or any tool.', BOLD)}")
    print("  SynthID and its relatives live in the image data and survive by design.")
    return 1 if failed else 0


# ==============================================================================================
# credit: put your name on it
# ==============================================================================================

def credit_main(argv=None):
    p = argparse.ArgumentParser(
        prog="credit",
        description="Put your name on your work, in the places that survive.",
        epilog="Only two layers reliably survive a social platform: the pixels, and a C2PA\n"
               "manifest on LinkedIn. Use --stamp if credit actually matters.\n"
               "See:  snitch --platforms",
        formatter_class=argparse.RawDescriptionHelpFormatter)
    _files(p)
    p.add_argument("--creator", help="the person who made it")
    p.add_argument("--credit", help="the studio or organisation to credit")
    p.add_argument("--copyright", help="copyright notice")
    p.add_argument("--licence", "--license", dest="licence",
                   help=f"one of: {', '.join(LICENCES)}")
    p.add_argument("--terms", help="usage terms, overrides the licence text")
    p.add_argument("--rights-url")
    p.add_argument("--url", help="licensor URL")
    p.add_argument("--contact", help="email or URL for licensing enquiries")
    p.add_argument("--title")
    p.add_argument("--description")
    p.add_argument("--keyword", action="append", default=[], help="repeatable")
    p.add_argument("--keep-gps", action="store_true",
                   help="keep location data. Off by default, because it doxxes people")

    g = p.add_argument_group("visible stamp, the only layer that survives everything")
    g.add_argument("--stamp", metavar="TEXT", help="burn this text into the pixels")
    g.add_argument("--stamp-sub", metavar="TEXT", help="second line under it")
    g.add_argument("--logo", help="PNG logo for the stamp")
    g.add_argument("--corner", default="bottom-right", choices=core.CORNERS)
    g.add_argument("--scale", type=float, default=0.05)
    g.add_argument("--opacity", type=float, default=0.85)
    g.add_argument("--font", help="path to a .ttf")
    g.add_argument("--quality", type=int, default=94)

    s = p.add_argument_group("C2PA Content Credentials, which LinkedIn displays")
    s.add_argument("--sign", action="store_true", help="also add a signed Content Credential")
    s.add_argument("--verify", action="store_true", help="only check an existing credential")
    s.add_argument("--key", help="PEM private key, generated on first use if absent")
    s.add_argument("--cert", help="PEM certificate")
    s.add_argument("--generated", action="store_true",
                   help="declare a generative model made this, not a camera")

    p.add_argument("--in-place", action="store_true")
    p.add_argument("-o", "--out")
    a = p.parse_args(argv)
    if not a.files:
        p.print_help()
        return 2

    if a.verify:
        for path in a.files:
            c = core.read_c2pa(path)
            if not c:
                print(f"  {os.path.basename(path)}: no Content Credential")
                continue
            man = (c.get("manifests") or {}).get(c.get("active_manifest"), {})
            sig = man.get("signature_info") or {}
            state = c.get("validation_state", "?")
            print(f"  {os.path.basename(path)}  {_c(state, GRN if state == 'Valid' else YEL)}  "
                  f"signed by {sig.get('issuer', '?')}  {man.get('title', '')}")
        return 0

    lic_name, lic_url = (None, None)
    if a.licence:
        if a.licence not in LICENCES:
            print(f"unknown licence {a.licence!r}. One of: {', '.join(LICENCES)}")
            return 2
        lic_name, lic_url = LICENCES[a.licence]
    terms = a.terms or (f"{lic_name}." if lic_name else None)
    if terms and a.contact:
        terms = f"{terms} Commercial licensing: contact {a.contact}."

    targets = []
    for path in a.files:
        if not os.path.exists(path):
            print(f"  {path}: not found")
            continue
        target = path
        if not a.in_place:
            target = _outpath(path, a.out if len(a.files) == 1 else None, "-credited")
            shutil.copy2(path, target)

        if a.stamp or a.logo:
            core.stamp(target, target, text=a.stamp or (a.creator or ""),
                       subtext=a.stamp_sub, logo=a.logo, corner=a.corner,
                       scale=a.scale, opacity=a.opacity, font=a.font, quality=a.quality)

        ok, err = core.write_credit(
            target, creator=a.creator, credit=a.credit, copyright_=a.copyright,
            terms=terms, rights_url=a.rights_url or lic_url, licensor=a.credit,
            licensor_url=a.url, contact=a.contact, title=a.title,
            description=a.description, keywords=a.keyword, drop_gps=not a.keep_gps)
        mark = _c("ok", GRN) if ok else _c("FAILED " + err, RED)
        stamped = " + visible stamp" if (a.stamp or a.logo) else ""
        print(f"  {os.path.basename(target)}  credit written{stamped}  {mark}")
        if a.stamp or a.logo:
            print(f"    {DIM}stamping re-encoded the image at quality {a.quality}{OFF}")
        targets.append(target)

    if a.sign and targets:
        from . import sign as signer
        ns = argparse.Namespace(files=targets, creator=a.creator, org=a.credit, url=a.url,
                                contact=a.contact, licence=a.licence, title=a.title,
                                description=a.description, key=a.key, cert=a.cert,
                                generated=a.generated)
        signer.run(ns)

    if not (a.stamp or a.logo):
        print(f"\n  {DIM}Most platforms strip what you just wrote. --stamp puts it in the"
              f" pixels,{OFF}")
        print(f"  {DIM}which is the only layer that always survives. snitch --platforms{OFF}")
    return 0


def main(argv=None):
    """Kept so `python -m snitch` still works."""
    return snitch_main(argv)


if __name__ == "__main__":
    sys.exit(snitch_main())
