"""What a PNG is saying in its text chunks, which is where generators put the whole prompt.

The strip path has always removed these. The read path did not report them, so the tool named for
telling you what your file says was silent about the most common carrier there is.
"""

import shutil
import struct
import zlib

import pytest
from PIL import Image, PngImagePlugin

from snitch import cli, core

EXIFTOOL = pytest.mark.skipif(not shutil.which("exiftool"), reason="ExifTool is not installed")


def comfy_png(path):
    """A PNG shaped like one that comes out of ComfyUI or the Stable Diffusion WebUI."""
    meta = PngImagePlugin.PngInfo()
    meta.add_text("parameters", "a cat, (ugly hands:1.4)\nSteps: 30, Sampler: Euler a")
    meta.add_text("workflow", '{"nodes":[{"id":1,"type":"KSampler"}]}')
    Image.new("RGB", (24, 18), "purple").save(path, pnginfo=meta)
    return path


def png_chunk(ctype, payload):
    crc = zlib.crc32(ctype + payload) & 0xFFFFFFFF
    return struct.pack(">I", len(payload)) + ctype + payload + struct.pack(">I", crc)


def insert_chunk(path, chunk):
    """Put a raw chunk in just before IEND, so Pillow's writer does not get a say in the bytes."""
    data = path.read_bytes()
    end = data.rindex(b"IEND") - 4
    path.write_bytes(data[:end] + chunk + data[end:])


def test_reads_text_chunks_a_generator_wrote(tmp_path):
    found = core.read_png_text(comfy_png(tmp_path / "gen.png"))

    keywords = {c["keyword"]: c for c in found}
    assert set(keywords) == {"parameters", "workflow"}
    assert keywords["parameters"]["chunk"] == "tEXt"
    assert "Sampler: Euler a" in keywords["parameters"]["text"]
    assert keywords["workflow"]["text"] == '{"nodes":[{"id":1,"type":"KSampler"}]}'


def test_reads_itxt_with_non_ascii_text(tmp_path):
    source = tmp_path / "itxt.png"
    meta = PngImagePlugin.PngInfo()
    meta.add_itxt("Description", "café naïve 中文 😀")
    Image.new("RGB", (8, 8), "blue").save(source, pnginfo=meta)

    found = core.read_png_text(source)

    assert len(found) == 1
    assert found[0]["chunk"] == "iTXt"
    assert found[0]["text"] == "café naïve 中文 😀"


def test_reads_compressed_ztxt_and_compressed_itxt(tmp_path):
    source = tmp_path / "compressed.png"
    Image.new("RGB", (8, 8), "red").save(source)
    long_prompt = "a very long prompt " * 40
    insert_chunk(source, png_chunk(
        b"zTXt", b"Comment\x00\x00" + zlib.compress(long_prompt.encode("latin-1"))))
    insert_chunk(source, png_chunk(
        b"iTXt", b"prompt\x00\x01\x00en\x00\x00" + zlib.compress("génératif".encode("utf-8"))))

    found = {c["keyword"]: c for c in core.read_png_text(source)}

    assert found["Comment"]["chunk"] == "zTXt"
    assert found["Comment"]["text"] == long_prompt
    assert found["prompt"]["chunk"] == "iTXt"
    assert found["prompt"]["text"] == "génératif"


def test_a_clean_png_and_a_jpeg_report_nothing_and_do_not_raise(tmp_path):
    clean = tmp_path / "clean.png"
    Image.new("RGB", (8, 8), "green").save(clean)
    jpeg = tmp_path / "photo.jpg"
    Image.new("RGB", (8, 8), "green").save(jpeg, "JPEG")

    assert core.read_png_text(clean) == []
    assert core.read_png_text(jpeg) == []


def test_a_truncated_png_reports_what_survived_instead_of_raising(tmp_path):
    source = comfy_png(tmp_path / "cut.png")
    data = source.read_bytes()
    source.write_bytes(data[:-9])

    found = core.read_png_text(source)

    assert [c["keyword"] for c in found] == ["parameters", "workflow"]


@EXIFTOOL
def test_inspect_names_the_generator_and_says_the_signal_is_not_a_credential(tmp_path):
    report = core.inspect(comfy_png(tmp_path / "gen.png"))

    assert report["generator_keywords"] == ["parameters", "workflow"]
    assert report["ai"] == "generative"
    assert report["ai_source"] == "png-text-chunk"


@EXIFTOOL
def test_a_caption_mentioning_a_model_is_not_treated_as_generated(tmp_path):
    source = tmp_path / "photo.png"
    meta = PngImagePlugin.PngInfo()
    meta.add_text("Description", "shot on film, not Stable Diffusion, no prompt involved")
    Image.new("RGB", (8, 8), "grey").save(source, pnginfo=meta)

    report = core.inspect(source)

    assert report["png_text"][0]["keyword"] == "Description"
    assert report["ai"] is None
    assert report["ai_source"] is None


@EXIFTOOL
def test_a_clean_png_reports_no_embedded_text(tmp_path):
    source = tmp_path / "clean.png"
    Image.new("RGB", (8, 8), "green").save(source)

    report = core.inspect(source)

    assert report["png_text"] == []
    assert report["generator_keywords"] == []
    assert report["ai_source"] is None


@EXIFTOOL
def test_no_comment_still_removes_the_chunks_without_touching_pixels(tmp_path):
    source = comfy_png(tmp_path / "gen.png")
    output = tmp_path / "gen-clean.png"

    removed = core.strip(str(source), str(output))

    assert removed > 0
    assert core.pixels_identical(str(source), str(output))
    assert core.read_png_text(output) == []
    assert core.inspect(output)["ai"] is None


def test_a_signed_credential_outranks_a_text_chunk(tmp_path, monkeypatch):
    """A chunk is plain text anyone can write. It may never overrule a bound claim."""
    source = comfy_png(tmp_path / "both.png")
    monkeypatch.setattr(core, "read_metadata", lambda _p: {"File:MIMEType": "image/png"})
    monkeypatch.setattr(core, "read_c2pa_report", lambda _p, _t=None: ("present", {
        "active_manifest": "urn:x",
        "manifests": {"urn:x": {"assertions": [
            {"label": "c2pa.actions",
             "data": {"actions": [{"digitalSourceType": "http://cv.iptc.org/newscodes/"
                                                        "digitalsourcetype/digitalCapture"}]}},
        ]}},
    }, ""))

    report = core.inspect(source)

    assert report["ai"] == "camera"
    assert report["ai_source"] == "c2pa"
    assert report["generator_keywords"] == ["parameters", "workflow"]


def test_a_hash_mismatch_is_reported_as_altered_not_merely_invalid():
    altered = {"validation_status": [{"code": "assertion.dataHash.mismatch"}]}
    untrusted = {"validation_status": [{"code": "signingCredential.untrusted"}]}

    assert cli._asset_altered(altered)
    assert not cli._identity_untrusted(altered)
    assert not cli._asset_altered(untrusted)
    assert cli._identity_untrusted(untrusted)
    assert cli._asset_altered({"validation_status": [
        {"code": "signingCredential.untrusted"}, {"code": "assertion.dataHash.mismatch"}]})
    assert not cli._asset_altered({})
