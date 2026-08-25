"""Regressions found by using the tool, not by testing its parts.

Both of these passed every unit test in the suite and both were wrong. They were found while
building a real probe image to upload to a real platform, which is the only exercise that runs the
commands in the order a person actually runs them.
"""

import shutil
import subprocess

import pytest
from PIL import Image

from snitch import cli, core

EXIFTOOL = pytest.mark.skipif(not shutil.which("exiftool"), reason="ExifTool is not installed")


def make_jpeg(path, size=(120, 90)):
    Image.new("RGB", size, "teal").save(path, "JPEG")


def gps_of(path):
    out = subprocess.run(
        ["exiftool", "-s3", "-GPSLatitude", "-n", str(path)],
        capture_output=True, text=True, check=False)
    return out.stdout.strip()


def camera_of(path):
    out = subprocess.run(
        ["exiftool", "-s3", "-EXIF:Make", str(path)],
        capture_output=True, text=True, check=False)
    return out.stdout.strip()


@EXIFTOOL
def test_stamp_does_not_silently_destroy_exif_when_keep_gps_is_asked_for(tmp_path):
    """--stamp re-encodes through Pillow, which writes a fresh file.

    Without carrying EXIF across, the stamp threw away the camera block and the GPS BEFORE the
    metadata step ran, which made --keep-gps a promise the tool could not keep."""
    source = tmp_path / "shot.jpg"
    make_jpeg(source)
    subprocess.run(["exiftool", "-q", "-overwrite_original",
                    "-GPSLatitude=45.9636", "-GPSLatitudeRef=N",
                    "-GPSLongitude=-66.6431", "-GPSLongitudeRef=W",
                    "-EXIF:Make=PROBE-CAM", str(source)], check=True)
    assert gps_of(source), "fixture should start with GPS"

    rc = cli.credit_main(["--creator", "Jane Doe", "--keep-gps", "--stamp", "TEST",
                          "--in-place", "--", str(source)])
    assert rc == 0
    assert gps_of(source), "--keep-gps must survive the stamp re-encode"
    assert camera_of(source) == "PROBE-CAM", "the camera block must survive the stamp re-encode"


@EXIFTOOL
def test_stamp_still_drops_gps_by_default(tmp_path):
    """The privacy default is the whole reason GPS handling is opinionated. Carrying EXIF through
    the stamp must not quietly turn that off."""
    source = tmp_path / "shot.jpg"
    make_jpeg(source)
    subprocess.run(["exiftool", "-q", "-overwrite_original",
                    "-GPSLatitude=45.9636", "-GPSLatitudeRef=N",
                    "-GPSLongitude=-66.6431", "-GPSLongitudeRef=W", str(source)], check=True)

    rc = cli.credit_main(["--creator", "Jane Doe", "--stamp", "TEST",
                          "--in-place", "--", str(source)])
    assert rc == 0
    assert not gps_of(source), "GPS must still be dropped when --keep-gps is not passed"


@EXIFTOOL
def test_sign_alone_does_not_fail_with_nothing_to_write(tmp_path):
    """`credit --sign --keep-gps` asks for no metadata and no GPS drop.

    ExifTool then had nothing to do, write_credit correctly said so, and the file was failed
    before it ever reached the signer. Signing a file and changing nothing else is a legitimate
    thing to want."""
    source = tmp_path / "shot.jpg"
    make_jpeg(source)

    args = ["--sign", "--digital-source", "camera", "--keep-gps", "--in-place", "--", str(source)]
    if not core.have("c2patool") and not core.have("exiftool"):
        pytest.skip("needs the toolchain")
    rc = cli.credit_main(args)
    # Without c2patool the run reports a missing dependency rather than "nothing to write".
    assert rc in (0, 3)
