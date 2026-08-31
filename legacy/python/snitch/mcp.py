"""A local stdio MCP server over the same core the three commands use.

WHY THIS IS A THIN ADAPTER AND NOTHING MORE. Every guarantee this tool makes about lossless
stripping, pixel identity and honest C2PA states is proved by the test suite against `core`. An
MCP server that reimplemented any of it would be a second implementation with none of that proof
behind it, and the two would drift. So every tool here resolves a path, calls core, and shapes the
answer. There is no image logic in this file.

STDOUT IS THE PROTOCOL WIRE. Nothing in this module may print. `core._run` captures ExifTool and
c2patool output rather than letting it through, and the CLI's colour and banner code is never
imported. Anything that needs to be said goes to stderr through `log`.

NOTHING IS MUTATED IN PLACE. `core.write_credit` edits the file it is given, so the credit tool
always copies to a new file first and edits the copy. A model calling a tool cannot be assumed to
have understood that its input was about to be overwritten, so it never is.
"""

import os
import shutil
import sys

from . import __version__, core, text

SUPPORTED_STRIP = "JPEG and PNG only. WebP, HEIC and AVIF are not supported for lossless stripping."


def log(message):
    """The only channel this server may write to besides the protocol itself."""
    print(f"snitch-mcp: {message}", file=sys.stderr, flush=True)


class ToolError(ValueError):
    """A refusal the caller should see as a message, not as a crash."""


def _source(path):
    """Resolve an input path, or refuse it. Ordinary readable files only."""
    if not path or not str(path).strip():
        raise ToolError("no path given")
    resolved = os.path.abspath(os.path.expanduser(str(path)))
    if not os.path.exists(resolved):
        raise ToolError(f"{resolved}: not found")
    if os.path.islink(resolved):
        # Following a link to read is fine. Writing through one is how a caller destroys something
        # it did not name, so the whole class is refused rather than special-cased per tool.
        raise ToolError(f"{resolved}: is a symbolic link. Pass the real file.")
    if not os.path.isfile(resolved):
        raise ToolError(f"{resolved}: not a regular file")
    return resolved


def _target(source, out_path, suffix, force):
    """Resolve an output path that cannot be the source and cannot be silently clobbered."""
    target = os.path.abspath(os.path.expanduser(str(out_path))) if out_path \
        else core.outpath(source, None, suffix)
    if os.path.isdir(target):
        raise ToolError(f"{target}: is a directory")
    if core.same_file(source, target):
        raise ToolError("output would overwrite the source. Give a different out_path.")
    if os.path.exists(target) and not force:
        raise ToolError(f"{target}: exists. Pass force=true to replace it.")
    parent = os.path.dirname(target) or "."
    if not os.path.isdir(parent):
        raise ToolError(f"{parent}: output directory does not exist")
    return target


def _removed_classes(before, after):
    """Name what actually went, by comparing the two reports rather than by assuming."""
    gone = []
    if before["gps"] and not after["gps"]:
        gone.append("location")
    if before["camera"] and not after["camera"]:
        gone.append("camera")
    if before["credit"] and not after["credit"]:
        gone.append("credit")
    if before["png_text"] and not after["png_text"]:
        gone.append("png text chunks")
    if before["c2pa_status"] in ("present", "detected-unverified") \
            and after["c2pa_status"] == "absent":
        gone.append("C2PA content credential")
    return gone


def c2pa_summary(report):
    """Integrity, trust and presence as three separate answers, because they are three facts."""
    status = report["c2pa_status"]
    c2pa = report["c2pa"] or {}
    if status != "present":
        return {
            "manifest": status,
            "integrity": "unknown",
            "signer_trusted": "unknown",
            "signer": None,
            "means": {
                "absent": "No content credential in this file. That is not evidence about how the "
                          "image was made: most files have never had one.",
                "detected-unverified": "A credential is in this file but c2patool is not installed, "
                                       "so it could not be validated. Install c2patool to read it.",
                "unavailable": "c2patool is not installed, so no credential could be read.",
                "error": report["c2pa_error"] or "the validator failed",
            }.get(status, status),
        }
    manifest = (c2pa.get("manifests") or {}).get(c2pa.get("active_manifest"), {})
    state = c2pa.get("validation_state", "unknown")
    altered = core.asset_altered(c2pa)
    untrusted = core.identity_untrusted(c2pa)
    return {
        "manifest": "present",
        "validation_state": state,
        "integrity": "altered" if altered else ("valid" if state == "Valid" else "unknown"),
        "signer_trusted": "no" if untrusted else ("yes" if state == "Valid" else "unknown"),
        "signer": (manifest.get("signature_info") or {}).get("issuer"),
        "title": manifest.get("title"),
        "validation_status": c2pa.get("validation_status") or [],
        "means": (
            "The pixels are not the ones that were signed: this image was altered after signing."
            if altered else
            "The asset binding holds, so the image matches its credential. "
            + ("The signing certificate is not on the validator trust list, so this says nothing "
               "about who signed it." if untrusted else
               "The signer is on the validator trust list.")
        ),
    }


def _refusals(fn):
    """Turn our refusals into the protocol's own error type, so a caller gets a sentence.

    Without this the SDK treats anything raised as a crash and wraps it in an unexpected-error
    envelope, which tells a model that the server is broken when in fact it declined on purpose.
    """
    import functools

    from mcp.server.mcpserver.exceptions import ToolError as ProtocolError

    @functools.wraps(fn)
    def wrapper(*args, **kwargs):
        try:
            return fn(*args, **kwargs)
        except (ToolError, core.ToolMissing) as exc:
            raise ProtocolError(str(exc)) from None
        except (OSError, ValueError) as exc:
            raise ProtocolError(str(exc)) from None

    return wrapper


def build_server():
    from mcp.server.mcpserver import MCPServer

    server = MCPServer(
        name="snitch",
        version=__version__,
        instructions="Read, strip and credit image metadata, and check text for hidden characters. "
                     "Everything runs locally. Nothing is uploaded and no network call is made.",
    )

    @server.tool(
        description="Report everything an image file is saying about its owner: GPS location, "
                    "camera, credit and copyright, PNG text chunks a generator wrote, and any "
                    "C2PA content credential. Reads the file, never changes it.")
    def snitch_inspect(path: str) -> dict:
        report = core.inspect(_source(path))
        report["c2pa_summary"] = c2pa_summary(report)
        return dict(report)

    @server.tool(
        description="Remove metadata from an image without re-encoding it, so the decoded pixels "
                    f"come out byte-identical. {SUPPORTED_STRIP} Writes a new file and leaves the "
                    "input untouched. Does not remove in-pixel watermarks such as SynthID: those "
                    "live in the image data and no metadata tool can reach them.")
    def snitch_strip_metadata(path: str, out_path: str = "", force: bool = False) -> dict:
        source = _source(path)
        before = core.inspect(source)
        target = _target(source, out_path, "-clean", force)
        removed, identical = core.strip_atomic(source, target)
        after = core.inspect(target)
        return {
            "output_path": target,
            "format": after["mime_type"],
            "bytes_before": before["bytes"],
            "bytes_after": after["bytes"],
            "bytes_removed": removed,
            "removed": _removed_classes(before, after),
            "pixels_identical": identical,
            "proof": "Decoded pixels compared by SHA-256 before and after. This is a per-run check, "
                     "not a claim about the algorithm.",
            "watermarks": "In-pixel watermarks are untouched. Metadata stripping cannot reach them.",
        }

    @server.tool(
        description="Write creator, credit, copyright and licence into an image's IPTC and XMP "
                    "fields, into a new file. Location data is dropped unless keep_gps is true, "
                    "because publishing coordinates deanonymises people. Does not sign a C2PA "
                    "credential and does not stamp the pixels.")
    def snitch_add_credit(path: str, creator: str = "", credit: str = "", copyright: str = "",
                          terms: str = "", rights_url: str = "", contact: str = "", title: str = "",
                          description: str = "", keep_gps: bool = False, out_path: str = "",
                          force: bool = False) -> dict:
        source = _source(path)
        fields = dict(creator=creator or None, credit=credit or None, copyright_=copyright or None,
                      terms=terms or None, rights_url=rights_url or None, contact=contact or None,
                      title=title or None, description=description or None)
        if not any(fields.values()):
            raise ToolError("nothing to write: give at least one of creator, credit, copyright, "
                            "terms, rights_url, contact, title or description")
        target = _target(source, out_path, "-credited", force)
        # Copy first, then edit the copy. write_credit mutates whatever file it is handed, and the
        # caller's input is not ours to change.
        temporary = core.temporary_sibling(target, keep_extension=True)
        try:
            shutil.copy2(source, temporary)
            core.write_credit(temporary, drop_gps=not keep_gps, **fields)
            os.replace(temporary, target)
        except BaseException:
            if os.path.exists(temporary):
                os.unlink(temporary)
            raise
        after = core.inspect(target)
        return {
            "output_path": target,
            "format": after["mime_type"],
            "bytes_before": os.path.getsize(source),
            "bytes_after": after["bytes"],
            "written": after["credit"],
            "gps_kept": bool(after["gps"]),
            "note": "Written into metadata, which survives ordinary copying but is removed by many "
                    "upload paths. A visible stamp is the version that survives stripping.",
        }

    @server.tool(
        description="Check an image's C2PA content credential. Reports presence, asset-binding "
                    "integrity and signer trust as three separate answers, because they are three "
                    "different facts. A missing credential is not evidence that an image is real, "
                    "and a self-signed certificate is not proof of who signed it.")
    def snitch_verify_c2pa(path: str) -> dict:
        report = core.inspect(_source(path))
        summary = c2pa_summary(report)
        summary["file"] = report["file"]
        return dict(summary)

    @server.tool(
        description="Find hidden characters in text: invisible zero-width marks, right-to-left "
                    "overrides, letters from mixed alphabets, unusual spaces and word-processor "
                    "substitutions. Returns the text with the tracking characters removed. Never "
                    "touches ZWJ or ZWNJ, which emoji sequences and Arabic, Persian, Kurdish and "
                    "Indic writing depend on.")
    def snitch_clean_text(text_input: str) -> dict:
        return dict(text.clean(text_input))

    for tool in server._tool_manager._tools.values():
        tool.fn = _refusals(tool.fn)

    return server


def main(argv=None):
    try:
        server = build_server()
    except ImportError:
        log("the MCP SDK is not installed. Install it with: pip install 'snitch-tools[mcp]'\n"
            "        It needs Python 3.10 or newer.")
        return 1
    log(f"serving on stdio, snitch {__version__}")
    server.run(transport="stdio")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
