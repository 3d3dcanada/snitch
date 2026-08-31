import os

from PIL import Image

from snitch import cli


def make_jpeg(path, colour="purple"):
    Image.new("RGB", (31, 19), colour).save(path, "JPEG", comment=b"private")


def test_multiple_files_honour_output_directory(tmp_path):
    first = tmp_path / "first.jpg"
    second = tmp_path / "second.jpg"
    output = tmp_path / "output"
    output.mkdir()
    make_jpeg(first)
    make_jpeg(second, "orange")

    result = cli.nocomment_main(["-o", str(output), str(first), str(second)])

    assert result == 0
    assert (output / "first-clean.jpg").is_file()
    assert (output / "second-clean.jpg").is_file()


def test_invalid_input_is_an_error_and_leaves_no_output(tmp_path, capsys):
    source = tmp_path / "not-an-image.jpg"
    source.write_text("not an image")

    result = cli.nocomment_main([str(source)])

    assert result == 1
    assert "not a JPEG" in capsys.readouterr().err
    assert not (tmp_path / "not-an-image-clean.jpg").exists()


def test_directory_is_rejected_without_output(tmp_path, capsys):
    source = tmp_path / "directory.jpg"
    source.mkdir()

    assert cli.nocomment_main([str(source)]) == 1
    assert "not a regular file" in capsys.readouterr().err
    assert not (tmp_path / "directory-clean.jpg").exists()


def test_in_place_uses_unique_temporary_and_preserves_existing_sentinel(tmp_path):
    source = tmp_path / "photo.jpg"
    old_temporary = tmp_path / "photo.jpg.tmp"
    make_jpeg(source)
    old_temporary.write_text("sentinel")

    result = cli.nocomment_main(["--in-place", str(source)])

    assert result == 0
    assert old_temporary.read_text() == "sentinel"
    assert Image.open(source).size == (31, 19)


def test_in_place_rejects_symlink_without_changing_target(tmp_path, capsys):
    target = tmp_path / "target.jpg"
    link = tmp_path / "link.jpg"
    make_jpeg(target)
    before = target.read_bytes()
    link.symlink_to(target)

    result = cli.nocomment_main(["--in-place", str(link)])

    assert result == 1
    assert link.is_symlink()
    assert target.read_bytes() == before
    assert "refusing in-place replacement of a symlink" in capsys.readouterr().err


def test_existing_output_requires_force(tmp_path, capsys):
    source = tmp_path / "photo.jpg"
    output = tmp_path / "elsewhere.jpg"
    make_jpeg(source)
    output.write_text("sentinel")

    refused = cli.nocomment_main(["-o", str(output), str(source)])
    assert refused == 1
    assert output.read_text() == "sentinel"
    assert "output exists" in capsys.readouterr().err

    replaced = cli.nocomment_main(["--force", "-o", str(output), str(source)])
    assert replaced == 0
    assert Image.open(output).size == (31, 19)


def test_failed_pixel_proof_does_not_replace_source(tmp_path, monkeypatch, capsys):
    source = tmp_path / "photo.jpg"
    make_jpeg(source)
    before = source.read_bytes()
    monkeypatch.setattr(cli.core, "pixels_identical", lambda _a, _b: False)

    result = cli.nocomment_main(["--in-place", str(source)])

    assert result == 1
    assert source.read_bytes() == before
    assert "refusing to write output: pixels changed" in capsys.readouterr().err
    assert not [name for name in os.listdir(tmp_path) if ".snitch-" in name]
