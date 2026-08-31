import os
import shutil

import pytest
from PIL import Image

from snitch import cli, core

EXIFTOOL = pytest.mark.skipif(not shutil.which("exiftool"), reason="ExifTool is not installed")


def make_jpeg(path):
    Image.new("RGB", (101, 67), "purple").save(path, "JPEG")


@EXIFTOOL
def test_unicode_credit_round_trip_survives_in_xmp_iptc_and_exif(tmp_path):
    source = tmp_path / "source.jpg"
    make_jpeg(source)

    ok, error = core.write_credit(
        source,
        creator="José Ángel",
        credit="Atelier Étoile",
        copyright_="© 2026 José Ángel",
        terms="Réutilisation autorisée.",
        rights_url="https://example.test/droits",
        licensor="Atelier Étoile",
        licensor_url="https://example.test",
        contact="josé@example.test",
        title="Crème brûlée",
        description="Illustración naïve — 東京",
        keywords=["café", "東京"],
    )

    assert ok, error
    metadata = core.read_metadata(source)
    assert metadata["XMP:Creator"] == "José Ángel"
    assert metadata["IPTC:By-line"] == "José Ángel"
    assert metadata["EXIF:Artist"] == "José Ángel"
    assert metadata["XMP:Rights"] == "© 2026 José Ángel"
    assert metadata["IPTC:CopyrightNotice"] == "© 2026 José Ángel"
    assert metadata["EXIF:Copyright"] == "© 2026 José Ángel"
    assert metadata["XMP:Title"] == "Crème brûlée"
    assert metadata["IPTC:ObjectName"] == "Crème brûlée"
    assert metadata["XMP:Description"] == "Illustración naïve — 東京"
    assert metadata["IPTC:Caption-Abstract"] == "Illustración naïve — 東京"
    assert metadata["XMP:Subject"] == ["café", "東京"]
    assert metadata["IPTC:Keywords"] == ["café", "東京"]
    assert metadata["IPTC:CodedCharacterSet"] == "\x1b%G"


@EXIFTOOL
def test_credit_cli_round_trip_all_fields_and_drops_gps(tmp_path):
    source = tmp_path / "photo José.jpg"
    make_jpeg(source)
    result = core._run([
        "exiftool", "-overwrite_original", "-GPSLatitude=45.9636", "-GPSLongitude=-66.6431",
        "-GPSLatitudeRef=N", "-GPSLongitudeRef=W", str(source),
    ])
    assert result.returncode == 0, result.stderr

    status = cli.credit_main([
        str(source), "--creator", "José Ángel", "--credit", "Atelier Étoile",
        "--copyright", "© 2026 José Ángel", "--licence", "cc-by-nc",
        "--url", "https://example.test", "--contact", "josé@example.test",
        "--title", "Crème brûlée", "--description", "Illustración naïve — 東京",
        "--keyword", "café", "--keyword", "東京",
    ])

    output = tmp_path / "photo José-credited.jpg"
    assert status == 0
    report = core.inspect(output)
    assert report["gps"] == {}
    assert report["credit"] == {
        "Creator": "José Ángel",
        "Credit": "Atelier Étoile",
        "Copyright": "© 2026 José Ángel",
        "Usage terms": "CC BY-NC 4.0. Commercial licensing: contact josé@example.test.",
        "Rights URL": "https://creativecommons.org/licenses/by-nc/4.0/",
        "Licensor": "Atelier Étoile",
        "Contact": "josé@example.test",
        "Title": "Crème brûlée",
        "Description": "Illustración naïve — 東京",
        "Keywords": ["café", "東京"],
    }


@EXIFTOOL
def test_png_stamp_preserves_existing_alpha(tmp_path):
    source = tmp_path / "alpha.png"
    Image.new("RGBA", (101, 67), (10, 20, 30, 40)).save(source)

    status = cli.credit_main([str(source), "--creator", "Artist", "--stamp", "Artist"])

    output = tmp_path / "alpha-credited.png"
    assert status == 0
    with Image.open(output) as image:
        assert image.mode == "RGBA"
        assert image.getpixel((0, 0))[3] == 40
        assert image.getchannel("A").getextrema()[1] > 40


@EXIFTOOL
def test_palette_png_stamp_preserves_transparency_as_rgba(tmp_path):
    source = tmp_path / "palette.png"
    image = Image.new("P", (101, 67), 0)
    image.putpalette([channel for value in range(256) for channel in (value, value, value)])
    image.info["transparency"] = 0
    image.save(source)

    status = cli.credit_main([str(source), "--creator", "Artist", "--stamp", "Artist"])

    output = tmp_path / "palette-credited.png"
    assert status == 0
    with Image.open(output) as stamped:
        assert stamped.mode == "RGBA"
        assert stamped.getpixel((0, 0))[3] == 0


@pytest.mark.parametrize("extension,format_name", [("webp", "WEBP"), ("tiff", "TIFF")])
def test_stamp_refuses_unsupported_formats_without_leaving_output(
        tmp_path, capsys, extension, format_name):
    source = tmp_path / f"source.{extension}"
    Image.new("RGB", (101, 67), "purple").save(source, format_name)

    status = cli.credit_main([str(source), "--creator", "Artist", "--stamp", "Artist"])

    assert status == 1
    assert "visible stamping supports JPEG and PNG only" in capsys.readouterr().err
    assert not (tmp_path / f"source-credited.{extension}").exists()


@pytest.mark.parametrize("extension,format_name", [("webp", "WEBP"), ("tiff", "TIFF")])
@EXIFTOOL
def test_metadata_only_credit_round_trips_in_other_verified_formats(
        tmp_path, extension, format_name):
    source = tmp_path / f"source.{extension}"
    Image.new("RGB", (101, 67), "purple").save(source, format_name)

    status = cli.credit_main([str(source), "--creator", "Format Artist"])

    assert status == 0
    assert core.inspect(tmp_path / f"source-credited.{extension}")["credit"]["Creator"] == (
        "Format Artist"
    )


@EXIFTOOL
def test_failed_write_is_nonzero_and_leaves_no_copy(tmp_path, capsys):
    source = tmp_path / "not-an-image.jpg"
    source.write_text("not an image")

    status = cli.credit_main([str(source), "--creator", "Artist"])

    assert status == 1
    assert "FAILED" in capsys.readouterr().err
    assert not (tmp_path / "not-an-image-credited.jpg").exists()
    assert not [name for name in os.listdir(tmp_path) if ".snitch-" in name]


def test_missing_exiftool_is_a_clean_error_without_partial_output(tmp_path, monkeypatch, capsys):
    source = tmp_path / "source.jpg"
    make_jpeg(source)
    monkeypatch.setattr(core, "have", lambda _tool: False)

    assert cli.credit_main(["--creator", "Artist", str(source)]) == 1
    error = capsys.readouterr().err
    assert "exiftool is not installed" in error
    assert "libimage-exiftool-perl" in error
    assert not (tmp_path / "source-credited.jpg").exists()


def test_directory_is_rejected_without_output(tmp_path, capsys):
    source = tmp_path / "directory.jpg"
    source.mkdir()

    assert cli.credit_main(["--creator", "Artist", str(source)]) == 1
    assert "not a regular file" in capsys.readouterr().err
    assert not (tmp_path / "directory-credited.jpg").exists()


def test_multiple_files_honour_output_directory(tmp_path):
    first = tmp_path / "first.jpg"
    second = tmp_path / "second.jpg"
    output = tmp_path / "output"
    output.mkdir()
    make_jpeg(first)
    make_jpeg(second)

    status = cli.credit_main([
        "-o", str(output), "--creator", "Artist", str(first), str(second),
    ])

    assert status == 0
    assert (output / "first-credited.jpg").is_file()
    assert (output / "second-credited.jpg").is_file()


def test_in_place_rejects_symlink_without_changing_target(tmp_path, capsys):
    target = tmp_path / "target.jpg"
    link = tmp_path / "link.jpg"
    make_jpeg(target)
    before = target.read_bytes()
    link.symlink_to(target)

    status = cli.credit_main(["--in-place", "--creator", "Artist", str(link)])

    assert status == 1
    assert link.is_symlink()
    assert target.read_bytes() == before
    assert "refusing in-place replacement of a symlink" in capsys.readouterr().err


def test_missing_logo_and_empty_operation_are_usage_errors(tmp_path):
    source = tmp_path / "source.jpg"
    make_jpeg(source)
    with pytest.raises(SystemExit) as missing_logo:
        cli.credit_main([str(source), "--logo", str(tmp_path / "missing.png")])
    assert missing_logo.value.code == 2
    with pytest.raises(SystemExit) as empty:
        cli.credit_main([str(source)])
    assert empty.value.code == 2
