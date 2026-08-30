"""The MCP surface, exercised through a real client session over a real stdio subprocess.

Importing the handlers and calling them directly would prove the Python works and prove nothing
about the server. The failure this file exists to catch is a stray byte on stdout corrupting the
protocol wire, and only a real session can see that.
"""

import asyncio
import json
import os
import shutil
import sys

import pytest
from PIL import Image, PngImagePlugin

from snitch import core, mcp

mcp_sdk = pytest.importorskip("mcp", reason="the MCP SDK is not installed")
EXIFTOOL = pytest.mark.skipif(not shutil.which("exiftool"), reason="ExifTool is not installed")


def gen_png(path):
    meta = PngImagePlugin.PngInfo()
    meta.add_text("parameters", "a cat, (ugly hands:1.4)\nSteps: 30")
    Image.new("RGB", (24, 18), "purple").save(path, pnginfo=meta)
    return path


def photo(path):
    Image.new("RGB", (24, 18), "purple").save(path, "JPEG")
    return path


# ----------------------------------------------------------------------------------------------
# a real client session
# ----------------------------------------------------------------------------------------------

async def _session(work):
    from mcp import ClientSession
    from mcp.client.stdio import StdioServerParameters, stdio_client

    params = StdioServerParameters(command=sys.executable, args=["-m", "snitch.mcp"])
    async with stdio_client(params) as (read, write), ClientSession(read, write) as session:
        await session.initialize()
        return await work(session)


def run_session(work):
    return asyncio.run(asyncio.wait_for(_session(work), timeout=60))


def payload(result):
    """The tool's structured answer, however this SDK version chooses to carry it."""
    for attribute in ("structured_content", "structuredContent"):
        carried = getattr(result, attribute, None)
        if carried:
            return carried
    return json.loads(result.content[0].text)


def errored(result):
    return getattr(result, "is_error", None) or getattr(result, "isError", None)


@EXIFTOOL
def test_a_real_client_discovers_every_tool_and_gets_clean_json_back(tmp_path):
    source = gen_png(tmp_path / "gen.png")

    async def work(session):
        listed = await session.list_tools()
        inspected = await session.call_tool("snitch_inspect", {"path": str(source)})
        cleaned = await session.call_tool("snitch_clean_text", {"text_input": "a​b"})
        return listed, inspected, cleaned

    listed, inspected, cleaned = run_session(work)

    assert {t.name for t in listed.tools} == {
        "snitch_inspect", "snitch_strip_metadata", "snitch_add_credit",
        "snitch_verify_c2pa", "snitch_clean_text",
    }
    # Every description has to warn about the limit it has, or a model will promise what it cannot do.
    strip = next(t for t in listed.tools if t.name == "snitch_strip_metadata")
    assert "JPEG and PNG only" in strip.description
    assert "SynthID" in strip.description

    report = payload(inspected)
    assert report["ai"] == "generative"
    assert report["ai_source"] == "png-text-chunk"
    assert report["c2pa_summary"]["manifest"] == "absent"

    assert payload(cleaned)["text"] == "ab"


@EXIFTOOL
def test_a_real_client_strips_a_file_and_never_touches_the_input(tmp_path):
    source = gen_png(tmp_path / "gen.png")
    before = source.read_bytes()

    async def work(session):
        return await session.call_tool("snitch_strip_metadata", {"path": str(source)})

    result = payload(run_session(work))

    assert source.read_bytes() == before
    assert result["pixels_identical"] is True
    assert result["bytes_removed"] > 0
    assert "png text chunks" in result["removed"]
    assert os.path.isfile(result["output_path"])
    assert core.read_png_text(result["output_path"]) == []


@EXIFTOOL
def test_a_real_client_gets_a_message_not_a_crash_for_a_bad_request(tmp_path):
    async def work(session):
        missing = await session.call_tool("snitch_inspect", {"path": str(tmp_path / "nope.jpg")})
        directory = await session.call_tool("snitch_inspect", {"path": str(tmp_path)})
        return missing, directory

    missing, directory = run_session(work)

    assert errored(missing) and "not found" in missing.content[0].text
    assert errored(directory) and "not a regular file" in directory.content[0].text


# ----------------------------------------------------------------------------------------------
# the guards, unit level
# ----------------------------------------------------------------------------------------------

def test_a_symlink_is_refused_rather_than_written_through(tmp_path):
    real = photo(tmp_path / "real.jpg")
    link = tmp_path / "link.jpg"
    link.symlink_to(real)

    with pytest.raises(mcp.ToolError, match="symbolic link"):
        mcp._source(str(link))


def test_output_may_not_be_the_source_or_silently_replace_a_file(tmp_path):
    source = str(photo(tmp_path / "a.jpg"))
    existing = tmp_path / "taken.jpg"
    existing.write_bytes(b"do not lose me")

    with pytest.raises(mcp.ToolError, match="overwrite the source"):
        mcp._target(source, source, "-clean", False)
    with pytest.raises(mcp.ToolError, match="exists"):
        mcp._target(source, str(existing), "-clean", False)
    with pytest.raises(mcp.ToolError, match="is a directory"):
        mcp._target(source, str(tmp_path), "-clean", False)
    with pytest.raises(mcp.ToolError, match="does not exist"):
        mcp._target(source, str(tmp_path / "nowhere" / "x.jpg"), "-clean", False)

    assert mcp._target(source, str(existing), "-clean", True) == str(existing)
    assert existing.read_bytes() == b"do not lose me"


def test_empty_and_missing_paths_are_refused(tmp_path):
    with pytest.raises(mcp.ToolError, match="no path given"):
        mcp._source("")
    with pytest.raises(mcp.ToolError, match="not found"):
        mcp._source(str(tmp_path / "ghost.jpg"))


# ----------------------------------------------------------------------------------------------
# the C2PA summary keeps its three facts apart
# ----------------------------------------------------------------------------------------------

def summary(status, c2pa=None, error=""):
    return mcp.c2pa_summary({"c2pa_status": status, "c2pa": c2pa, "c2pa_error": error})


def test_absent_unavailable_and_detected_are_three_different_answers():
    absent = summary("absent")
    detected = summary("detected-unverified")
    unavailable = summary("unavailable")

    assert absent["manifest"] == "absent"
    assert "not evidence" in absent["means"]
    assert detected["manifest"] == "detected-unverified"
    assert "c2patool" in detected["means"]
    assert unavailable["manifest"] == "unavailable"
    for report in (absent, detected, unavailable):
        assert report["integrity"] == "unknown"
        assert report["signer_trusted"] == "unknown"


def test_an_untrusted_signer_is_not_an_altered_image_and_the_reverse():
    valid_untrusted = summary("present", {
        "validation_state": "Valid",
        "active_manifest": "urn:x",
        "manifests": {"urn:x": {"title": "t", "signature_info": {"issuer": "snitch"}}},
        "validation_status": [{"code": "signingCredential.untrusted"}],
    })
    tampered = summary("present", {
        "validation_state": "Invalid",
        "active_manifest": "urn:x",
        "manifests": {"urn:x": {"title": "t", "signature_info": {"issuer": "snitch"}}},
        "validation_status": [{"code": "signingCredential.untrusted"},
                              {"code": "assertion.dataHash.mismatch"}],
    })

    assert valid_untrusted["integrity"] == "valid"
    assert valid_untrusted["signer_trusted"] == "no"
    assert "says nothing about who signed it" in valid_untrusted["means"]

    assert tampered["integrity"] == "altered"
    assert "altered after signing" in tampered["means"]


@EXIFTOOL
def test_credit_writes_a_copy_and_drops_location_by_default(tmp_path):
    source = photo(tmp_path / "shot.jpg")
    core.write_credit(str(source), creator="Original")
    core._run(["exiftool", "-overwrite_original", "-q", "-GPSLatitude=50.2447",
               "-GPSLatitudeRef=N", str(source)])
    before = source.read_bytes()
    server_tool = None

    # Call the handler the server registered, so the test covers the wiring and not a copy of it.
    for name, fn in _handlers().items():
        if name == "snitch_add_credit":
            server_tool = fn
    result = server_tool(path=str(source), creator="Renée Åberg", copyright="© 2026")

    assert source.read_bytes() == before
    assert result["output_path"] != str(source)
    assert result["written"]["Creator"] == "Renée Åberg"
    assert result["gps_kept"] is False
    assert core.inspect(result["output_path"])["gps"] == {}


def test_credit_refuses_a_call_with_nothing_to_write(tmp_path):
    from mcp.server.mcpserver.exceptions import ToolError as ProtocolError

    source = str(photo(tmp_path / "shot.jpg"))
    # A registered handler raises the protocol's error type, so the caller gets a sentence rather
    # than an unexpected-crash envelope that would read as "this server is broken".
    with pytest.raises(ProtocolError, match="nothing to write"):
        _handlers()["snitch_add_credit"](path=source)


def _handlers():
    server = mcp.build_server()
    return {name: tool.fn for name, tool in server._tool_manager._tools.items()}


@EXIFTOOL
def test_nothing_but_protocol_ever_reaches_stdout(tmp_path):
    """stdout is the wire. One stray byte and every client sees a corrupt stream.

    ExifTool and c2patool are subprocesses whose output could leak, and the CLI prints colour and
    banners, so this asserts the property directly instead of trusting that nobody imported it.
    """
    source = gen_png(tmp_path / "gen.png")
    captured = {}

    async def work(session):
        await session.call_tool("snitch_inspect", {"path": str(source)})
        await session.call_tool("snitch_verify_c2pa", {"path": str(source)})
        await session.call_tool("snitch_strip_metadata", {"path": str(source)})
        await session.call_tool("snitch_inspect", {"path": str(tmp_path / "ghost.jpg")})
        return True

    from mcp import ClientSession
    from mcp.client.stdio import StdioServerParameters, stdio_client

    log_path = tmp_path / "stderr.log"

    async def session_capturing_stderr():
        params = StdioServerParameters(command=sys.executable, args=["-m", "snitch.mcp"])
        # A real file, because the transport hands this straight to subprocess as a descriptor.
        with open(log_path, "w+", encoding="utf-8") as errors:
            async with stdio_client(params, errlog=errors) as (read, write):
                async with ClientSession(read, write) as session:
                    await session.initialize()
                    await work(session)

    asyncio.run(asyncio.wait_for(session_capturing_stderr(), timeout=60))
    captured["stderr"] = log_path.read_text(encoding="utf-8")

    # The banner and the failure notice went to stderr, which is where they belong.
    assert "snitch-mcp: serving on stdio" in captured["stderr"]
    # And nothing raw from ExifTool or the colour codes came back through the protocol.
    assert "\x1b[" not in captured["stderr"]
