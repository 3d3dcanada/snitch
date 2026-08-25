import json
import os
import subprocess
import sys

import pytest
from PIL import Image

from snitch import __version__, cli, core


def make_jpeg(path):
    Image.new("RGB", (31, 19), "purple").save(path, "JPEG")


def test_invalid_and_missing_files_return_failure(tmp_path, capsys):
    invalid = tmp_path / "text renamed.jpg"
    missing = tmp_path / "missing.jpg"
    invalid.write_text("not an image")

    result = cli.snitch_main([str(invalid), str(missing)])

    assert result == 1
    errors = capsys.readouterr().err
    assert "not an image (ExifTool identified TXT)" in errors
    assert "not found" in errors


def test_human_output_quotes_paths_for_copy_and_paste(tmp_path, capsys):
    source = tmp_path / "photo José with spaces.jpg"
    make_jpeg(source)

    assert cli.snitch_main([str(source)]) == 0

    output = capsys.readouterr().out
    quoted = f"-- '{source}'"
    assert f"credit --creator \"Your Name\" {quoted}" in output
    assert "\x1b[" not in output


def test_json_output_is_stable_unicode_and_reports_errors(tmp_path, capsys):
    valid = tmp_path / "photo José.jpg"
    missing = tmp_path / "missing.jpg"
    make_jpeg(valid)

    result = cli.snitch_main(["--json", str(valid), str(missing)])

    assert result == 1
    payload = json.loads(capsys.readouterr().out)
    assert set(payload) == {"files"}
    assert payload["files"][0]["file"] == "photo José.jpg"
    assert payload["files"][0]["mime_type"] == "image/jpeg"
    assert payload["files"][1] == {"path": str(missing), "error": "not found"}


def test_platform_json_includes_notes_and_check_instructions(capsys):
    assert cli.snitch_main(["--platforms", "--notes", "--check", "--json"]) == 0

    payload = json.loads(capsys.readouterr().out)["platforms"]
    assert payload["platforms"]["LinkedIn"]["c2pa"]["note"]
    assert "Upload it" in payload["how_to_verify"]


def test_inspect_marks_detected_manifest_unverified_without_c2patool(tmp_path, monkeypatch):
    source = tmp_path / "signed-looking.jpg"
    make_jpeg(source)
    original_read = core.read_metadata

    def fake_metadata(path):
        metadata = original_read(path)
        metadata["JUMBF:JUMDLabel"] = "c2pa"
        return metadata

    monkeypatch.setattr(core, "read_metadata", fake_metadata)

    report = core.inspect(source, c2patool="/definitely/missing/c2patool")

    assert report["c2pa_status"] == "detected-unverified"
    assert report["c2pa"] is None


def test_python_module_entrypoint_and_version_work():
    result = subprocess.run(
        [sys.executable, "-m", "snitch", "--version"], capture_output=True, text=True,
        cwd=os.path.dirname(os.path.dirname(__file__)), check=False,
    )

    assert result.returncode == 0
    assert result.stdout.strip() == f"snitch {__version__}"


@pytest.mark.parametrize("value", ["", "1"])
def test_no_color_convention_disables_ansi_for_tty(monkeypatch, value):
    class Tty:
        @staticmethod
        def isatty():
            return True

    monkeypatch.setattr(cli.sys, "stdout", Tty())
    monkeypatch.setenv("NO_COLOR", value)
    assert cli._colour_enabled() is False
