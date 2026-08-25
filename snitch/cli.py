"""Three tools, three commands. You type the name of the thing you want.

    snitch      photo.jpg              what is this file telling people about you
    no-comment  photo.jpg              make it stop
    credit      photo.jpg --creator    put your name on it, so it stays

There is deliberately no umbrella command with these hidden underneath as subcommands. Each tool
is the thing it is called, because the name IS the interface: you remember "snitch" long after you
have forgotten which flag of which subcommand read GPS.
"""

import argparse
import contextlib
import json
import math
import os
import shlex
import shutil
import sys
import tempfile

from . import __version__, core, survival
from .core import LICENCES

BOLD = "\033[1m"
DIM = "\033[2m"
RED = "\033[31m"
GRN = "\033[32m"
YEL = "\033[33m"
OFF = "\033[0m"


def _colour_enabled():
    if "NO_COLOR" in os.environ or not sys.stdout.isatty():
        return False
    if os.name != "nt":
        return True
    return bool(os.environ.get("WT_SESSION") or os.environ.get("ANSICON")
                or os.environ.get("ConEmuANSI") == "ON"  # noqa: SIM112 - ConEmu's real spelling
                or os.environ.get("TERM", "").lower() not in ("", "dumb"))


def _c(s, colour):
    return f"{colour}{s}{OFF}" if _colour_enabled() else s


def _command_path(path):
    return f"-- {shlex.quote(path)}"


def _identity_untrusted(c2pa_report):
    return any(
        status.get("code") == "signingCredential.untrusted"
        for status in (c2pa_report.get("validation_status") or [])
        if isinstance(status, dict)
    )


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


def _temporary_sibling(path, keep_extension=False):
    directory = os.path.dirname(os.path.abspath(path))
    basename = os.path.basename(path)
    stem, extension = os.path.splitext(basename)
    prefix = f".{stem if keep_extension else basename}.snitch-"
    suffix = extension if keep_extension else ".tmp"
    fd, temporary = tempfile.mkstemp(prefix=prefix, suffix=suffix, dir=directory)
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
        with contextlib.suppress(FileNotFoundError):
            os.unlink(temporary)


def _files(sp):
    sp.add_argument("files", nargs="*", help="image files")
    sp.add_argument("--version", action="version", version=f"%(prog)s {__version__}")


# ==============================================================================================
# snitch: what is this file telling people
# ==============================================================================================

def print_platforms(notes=False, check=False):
    print(f"\n{_c('Evidence on platform metadata handling', BOLD)}"
          f"   {_c(f'researched {survival.RESEARCHED}', DIM)}\n")
    print("  D documented by platform   C independently corroborated   ? unverified\n")
    width = max(len(p) for p in survival.PLATFORMS) + 2
    head = "".join(f"{lbl:<26}" for _, lbl, _ in survival.LAYERS)
    print(f"  {'':<{width}}{head}")
    for name, layers in survival.PLATFORMS.items():
        row = f"  {name:<{width}}"
        for key, _, _ in survival.LAYERS:
            record = layers[key]
            verdict = record["verdict"]
            colour = (DIM if record["evidence"] == survival.INFERENCE else
                      {survival.KEEP: GRN, survival.STRIP: RED,
                       survival.PARTIAL: YEL, survival.READS: GRN}.get(verdict, DIM))
            row += _c(f"{survival.display_cell(record):<26}", colour)
        print(row)
    print(f"\n  {_c(survival.one_line_advice(), BOLD)}\n")
    if notes:
        for name, layers in survival.PLATFORMS.items():
            print(f"  {_c(name, BOLD)}")
            for key, lbl, _ in survival.LAYERS:
                record = layers[key]
                note = record["note"]
                if note:
                    print(f"    {lbl:<26} [{record['evidence']}] {note}")
                    for source in record["sources"]:
                        print(f"      source: {source['title']} — {source['url']}")
            print()
    else:
        print(f"  {_c('--notes for the detail on every cell', DIM)}")
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
                   help="sourced platform metadata-handling research")
    p.add_argument("--notes", action="store_true", help="detail behind every cell")
    p.add_argument("--check", action="store_true", help="how to verify a row yourself")
    p.add_argument("--json", dest="json_output", action="store_true",
                   help="emit stable machine-readable JSON")
    a = p.parse_args(argv)

    if a.json_output:
        result = {}
        failed = False
        if a.platforms or not a.files:
            result["platforms"] = survival.as_dict(a.notes, a.check)
        if a.files:
            reports = []
            for path in a.files:
                if not os.path.exists(path):
                    reports.append({"path": os.path.abspath(path), "error": "not found"})
                    failed = True
                    continue
                if not os.path.isfile(path):
                    reports.append({"path": os.path.abspath(path), "error": "not a regular file"})
                    failed = True
                    continue
                try:
                    report = core.inspect(path)
                except (OSError, ValueError, core.ToolMissing) as exc:
                    reports.append({"path": os.path.abspath(path), "error": str(exc)})
                    failed = True
                    continue
                reports.append(report)
                failed = failed or report["c2pa_status"] == "error"
            result["files"] = reports
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 1 if failed else 0

    if a.platforms or not a.files:
        print_platforms(a.notes, a.check)
        if not a.files:
            return 0

    failed = False
    for path in a.files:
        if not os.path.exists(path):
            print(f"  {path}: not found", file=sys.stderr)
            failed = True
            continue
        if not os.path.isfile(path):
            print(f"  {path}: not a regular file", file=sys.stderr)
            failed = True
            continue
        try:
            r = core.inspect(path)
        except (OSError, ValueError, core.ToolMissing) as exc:
            print(f"  {path}: {exc}", file=sys.stderr)
            failed = True
            continue
        print(f"\n{_c(r['file'], BOLD)}  {r['bytes']:,} bytes")

        if r["gps"]:
            print(_c("  LOCATION IS IN THIS FILE", RED))
            for k, v in r["gps"].items():
                print(f"    {k.split(':')[-1]:<16} {v}")
            print(_c("    Anyone who downloads this can see where it was taken.", RED))
            print(f"    Make it stop:  no-comment {_command_path(path)}")
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
                rendered = str(v)
                if len(rendered) > 90:
                    rendered = rendered[:89] + "…"
                print(f"    {k:<16} {rendered}")
        else:
            print(_c("  NO CREDIT AT ALL", YEL))
            print("    Nothing in this file says who made it.")
            print(f"    Fix it:  credit --creator \"Your Name\" {_command_path(path)}")

        if r["c2pa"]:
            c = r["c2pa"]
            man = (c.get("manifests") or {}).get(c.get("active_manifest"), {})
            sig = man.get("signature_info") or {}
            print(f"  C2PA Content Credential  [{c.get('validation_state', '?')}]")
            print(f"    title            {man.get('title', '?')}")
            print(f"    certificate issuer {sig.get('issuer', '?')}")
            if _identity_untrusted(c):
                print(_c("    SIGNER IDENTITY UNTRUSTED (certificate not on validator trust list)", YEL))
            if r["ai"] == "generative":
                print(_c("    THIS SAYS IT WAS MADE BY A GENERATIVE MODEL", YEL))
            elif r["ai"] == "camera":
                print(_c("    asserts a camera made this, not a model", GRN))
        else:
            status = r["c2pa_status"]
            if status == "absent":
                print("  no C2PA Content Credential")
            elif status == "detected-unverified":
                print(_c("  C2PA Content Credential detected; validation unavailable", YEL))
                print("    Install c2patool to read and validate it.")
            elif status == "unavailable":
                print(_c("  C2PA status unavailable: c2patool is not installed", YEL))
            else:
                print(_c(f"  C2PA check failed: {r['c2pa_error']}", RED))
                failed = True

    print(f"\n  {_c('snitch --platforms   what survives an upload and what does not', DIM)}")
    return 1 if failed else 0


# ==============================================================================================
# no-comment: make it stop
# ==============================================================================================

def nocomment_main(argv=None):
    p = argparse.ArgumentParser(
        prog="no-comment",
        description="Strip metadata out of an image. Losslessly: the pixels do not change.",
        epilog="What this does NOT do: in-pixel watermarks such as Google SynthID are part of\n"
               "the image itself. Metadata stripping does not remove them; attempts to do so\n"
               "repaint pixels and cannot prove the watermark is gone.",
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
        if not os.path.isfile(path):
            print(f"  {path}: not a regular file", file=sys.stderr)
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

    print(f"\n  {_c('In-pixel watermarks are not touched by metadata stripping.', BOLD)}")
    print("  SynthID and its relatives live in image data; removal attempts repaint pixels.")
    return 1 if failed else 0


# ==============================================================================================
# credit: put your name on it
# ==============================================================================================

def credit_main(argv=None):
    p = argparse.ArgumentParser(
        prog="credit",
        description="Put your name on your work, in the places that survive.",
        epilog="Pixels are the only broadly portable layer. Platform handling of embedded\n"
               "metadata varies by route and date; see the sourced snitch --platforms table.",
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

    g = p.add_argument_group("visible stamp; portable but still subject to crop and re-encoding")
    g.add_argument("--stamp", metavar="TEXT", help="burn this text into the pixels")
    g.add_argument("--stamp-sub", metavar="TEXT", help="second line under it")
    g.add_argument("--logo", help="PNG logo for the stamp")
    g.add_argument("--corner", default="bottom-right", choices=core.CORNERS)
    g.add_argument("--scale", type=float, default=0.05)
    g.add_argument("--opacity", type=float, default=0.85)
    g.add_argument("--font", help="path to a .ttf")
    g.add_argument("--quality", type=int, default=94)

    s = p.add_argument_group("C2PA Content Credentials; platform and trust support varies")
    s.add_argument("--sign", action="store_true",
                   help="also add a development-grade self-signed Content Credential")
    s.add_argument("--verify", action="store_true", help="only check an existing credential")
    s.add_argument("--key", help="PEM private key, generated on first use if absent")
    s.add_argument("--cert", help="PEM certificate")
    s.add_argument("--digital-source", choices=("camera", "digital", "screen", "human-edited",
                                                 "generated", "ai-edited", "algorithmic",
                                                 "data-driven"),
                   help="how the signed asset was created; required with --sign")
    s.add_argument("--generated", action="store_true",
                   help="deprecated alias for --digital-source generated")

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

    metadata_requested = any((a.creator, a.credit, a.copyright, a.licence, a.terms,
                              a.rights_url, a.url, a.contact, a.title, a.description,
                              a.keyword))
    stamp_requested = bool(a.stamp or a.logo)
    modifying_options = metadata_requested or stamp_requested or a.sign or a.generated
    if a.verify and (modifying_options or a.in_place or a.out or a.force or a.keep_gps
                     or a.key or a.cert or a.stamp_sub or a.font):
        p.error("--verify cannot be combined with writing, stamping, or output options")
    if not a.verify and not (metadata_requested or stamp_requested or a.sign):
        p.error("nothing to do; provide credit fields, --stamp/--logo, or --sign")
    if a.generated and not a.sign:
        p.error("--generated only applies with --sign")
    if a.digital_source and not a.sign:
        p.error("--digital-source only applies with --sign")
    if a.generated and a.digital_source:
        p.error("use --generated or --digital-source generated, not both")
    if a.sign and not (a.generated or a.digital_source):
        p.error("--sign requires --digital-source so the credential does not guess")
    if (a.key or a.cert) and not a.sign:
        p.error("--key and --cert only apply with --sign")
    if bool(a.key) != bool(a.cert):
        p.error("--key and --cert must be provided together")
    if a.stamp_sub and not stamp_requested:
        p.error("--stamp-sub requires --stamp or --logo")
    if a.logo and not os.path.isfile(a.logo):
        p.error(f"logo not found: {a.logo}")
    if a.font and not os.path.isfile(a.font):
        p.error(f"font not found: {a.font}")
    if not math.isfinite(a.scale) or not 0 < a.scale <= 1:
        p.error("--scale must be greater than 0 and at most 1")
    if not math.isfinite(a.opacity) or not 0 < a.opacity <= 1:
        p.error("--opacity must be greater than 0 and at most 1")
    if not 1 <= a.quality <= 100:
        p.error("--quality must be between 1 and 100")
    if a.in_place and a.force:
        p.error("--force is only meaningful with copied output")

    if a.verify:
        failed = False
        for path in a.files:
            if not os.path.exists(path):
                print(f"  {path}: not found", file=sys.stderr)
                failed = True
                continue
            if not os.path.isfile(path):
                print(f"  {path}: not a regular file", file=sys.stderr)
                failed = True
                continue
            status, c, error = core.read_c2pa_report(path)
            if status == "absent":
                print(f"  {os.path.basename(path)}: no Content Credential")
                failed = True
                continue
            if not c:
                print(f"  {os.path.basename(path)}: C2PA check failed: {error}", file=sys.stderr)
                failed = True
                continue
            man = (c.get("manifests") or {}).get(c.get("active_manifest"), {})
            sig = man.get("signature_info") or {}
            state = c.get("validation_state", "?")
            print(f"  {os.path.basename(path)}  {_c(state, GRN if state == 'Valid' else YEL)}  "
                  f"certificate issuer {sig.get('issuer', '?')}  {man.get('title', '')}")
            if _identity_untrusted(c):
                print(_c("    SIGNER IDENTITY UNTRUSTED (certificate not on validator trust list)", YEL))
            if state != "Valid":
                failed = True
        return 1 if failed else 0

    lic_name, lic_url = (None, None)
    if a.licence:
        if a.licence not in LICENCES:
            print(f"unknown licence {a.licence!r}. One of: {', '.join(LICENCES)}")
            return 2
        lic_name, lic_url = LICENCES[a.licence]
    terms = a.terms or (f"{lic_name}." if lic_name else None)
    if terms and a.contact:
        terms = f"{terms} Commercial licensing: contact {a.contact}."

    try:
        outputs = _output_paths(a.files, a.out, "-credited")
    except ValueError as e:
        p.error(str(e))

    targets = []
    failed = False
    for path, planned_output in zip(a.files, outputs):
        if not os.path.exists(path):
            print(f"  {path}: not found", file=sys.stderr)
            failed = True
            continue
        if not os.path.isfile(path):
            print(f"  {path}: not a regular file", file=sys.stderr)
            failed = True
            continue
        if a.in_place and os.path.islink(path):
            print(f"  {path}: refusing in-place replacement of a symlink", file=sys.stderr)
            failed = True
            continue
        target = path if a.in_place else planned_output
        if not a.in_place and _same_file(path, target):
            print(f"  {path}: output is the input; use --in-place", file=sys.stderr)
            failed = True
            continue
        if not a.in_place and os.path.lexists(target) and not a.force:
            print(f"  {target}: output exists; pass --force to replace it", file=sys.stderr)
            failed = True
            continue

        temporary = None
        try:
            temporary = _temporary_sibling(target, keep_extension=True)
            shutil.copy2(path, temporary)
            if stamp_requested:
                core.stamp(temporary, temporary, text=a.stamp or (a.creator or ""),
                           subtext=a.stamp_sub, logo=a.logo, corner=a.corner,
                           scale=a.scale, opacity=a.opacity, font=a.font, quality=a.quality)

            ok, err = core.write_credit(
                temporary, creator=a.creator, credit=a.credit, copyright_=a.copyright,
                terms=terms, rights_url=a.rights_url or lic_url, licensor=a.credit,
                licensor_url=a.url, contact=a.contact, title=a.title,
                description=a.description, keywords=a.keyword, drop_gps=not a.keep_gps)
            if not ok:
                raise ValueError(err or "ExifTool did not write the requested metadata")
            os.replace(temporary, target)
            temporary = None
        except (OSError, ValueError, core.ToolMissing) as e:
            print(f"  {os.path.basename(path)}: FAILED {e}", file=sys.stderr)
            failed = True
            continue
        finally:
            if temporary:
                with contextlib.suppress(FileNotFoundError):
                    os.unlink(temporary)

        mark = _c("ok", GRN)
        stamped = " + visible stamp" if (a.stamp or a.logo) else ""
        print(f"  {os.path.basename(target)}  credit written{stamped}  {mark}")
        if stamp_requested:
            if os.path.splitext(target)[1].lower() in (".jpg", ".jpeg"):
                print(f"    stamping re-encoded the JPEG at quality {a.quality}")
            else:
                print("    stamping rewrote the PNG pixel data")
        targets.append(target)

    if a.sign and targets:
        from . import sign as signer
        ns = argparse.Namespace(files=targets, creator=a.creator, org=a.credit, url=a.url,
                                contact=a.contact, licence=a.licence, title=a.title,
                                description=a.description, key=a.key, cert=a.cert,
                                generated=a.generated, digital_source=a.digital_source)
        if signer.run(ns):
            failed = True

    if not stamp_requested:
        print(f"\n  {_c('Platform handling varies. --stamp puts credit in the pixels, where it', DIM)}")
        print(f"  {_c('survives metadata stripping but may still be cropped. snitch --platforms', DIM)}")
    return 1 if failed else 0


def main(argv=None):
    """Kept so `python -m snitch` still works."""
    return snitch_main(argv)


if __name__ == "__main__":
    sys.exit(snitch_main())
