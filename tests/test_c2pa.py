import argparse
import json
import os
import shutil
import subprocess

import pytest
from PIL import Image

from snitch import core, sign


C2PATOOL = pytest.mark.skipif(not sign.c2patool(), reason="c2patool is not installed")
OPENSSL = pytest.mark.skipif(not shutil.which("openssl"), reason="OpenSSL is not installed")


def make_jpeg(path):
    Image.new("RGB", (101, 67), "purple").save(path, "JPEG")


def action_data(manifest):
    return next(item["data"] for item in manifest["assertions"]
                if item["label"] == "c2pa.actions")["actions"][0]


def test_manifest_does_not_claim_camera_capture_without_user_evidence():
    with pytest.raises(ValueError, match="digital source is required"):
        sign.manifest(title="Ordinary file")
    normal = sign.manifest(title="Ordinary file", digital_source="digital")
    generated = sign.manifest(title="Generated file", generated=True)

    assert action_data(normal)["digitalSourceType"] == sign.DIGITAL_SOURCES["digital"]
    assert action_data(generated)["digitalSourceType"] == sign.GENERATED


def test_c2pa_report_distinguishes_unavailable_absent_and_errors(monkeypatch):
    monkeypatch.setattr(core, "resolve_c2patool", lambda _tool=None: None)
    assert core.read_c2pa_report("photo.jpg") == (
        "unavailable", None, "c2patool is not installed"
    )

    monkeypatch.setattr(core, "resolve_c2patool", lambda _tool=None: "/fake/c2patool")
    monkeypatch.setattr(core, "_run", lambda _cmd: subprocess.CompletedProcess(
        _cmd, 1, "", "Error: No claim found\n"
    ))
    assert core.read_c2pa_report("photo.jpg") == ("absent", None, "")

    monkeypatch.setattr(core, "_run", lambda _cmd: subprocess.CompletedProcess(
        _cmd, 1, "", "Error: Unsupported file type\n"
    ))
    status, report, error = core.read_c2pa_report("photo.jpg")
    assert status == "error"
    assert report is None
    assert error == "Error: Unsupported file type"


def test_config_directory_uses_native_platform_conventions():
    assert sign.config_dir(
        os_name="nt", platform_name="win32", environ={"APPDATA": "C:/Users/A/AppData/Roaming"},
        home="C:/Users/A",
    ) == "C:/Users/A/AppData/Roaming/snitch"
    assert sign.config_dir(
        os_name="posix", platform_name="darwin", environ={}, home="/Users/a",
    ) == "/Users/a/Library/Application Support/snitch"
    assert sign.config_dir(
        os_name="posix", platform_name="linux", environ={"XDG_CONFIG_HOME": "/config"},
        home="/home/a",
    ) == "/config/snitch"


@OPENSSL
def test_certificate_generation_handles_unicode_and_refuses_half_a_pair(tmp_path):
    key = tmp_path / "relative" / "key.pem"
    cert = tmp_path / "other" / "cert.pem"

    assert sign.ensure_cert(key, cert, "Atelier Étoile / Studio") is True
    assert oct(key.stat().st_mode & 0o777) == "0o600"
    subject = core._run([
        "openssl", "x509", "-in", str(cert), "-noout", "-subject", "-nameopt", "utf8",
    ])
    assert subject.returncode == 0
    assert "Atelier Étoile / Studio" in subject.stdout
    assert sign.ensure_cert(key, cert, "ignored") is False

    cert.unlink()
    before = key.read_bytes()
    with pytest.raises(RuntimeError, match="refusing to replace existing"):
        sign.ensure_cert(key, cert, "ignored")
    assert key.read_bytes() == before
    assert not cert.exists()


def test_sign_file_passes_key_paths_not_private_material_in_environment(
        tmp_path, monkeypatch):
    source = tmp_path / "source.jpg"
    key = tmp_path / "key.pem"
    cert = tmp_path / "cert.pem"
    make_jpeg(source)
    key.write_text("PRIVATE SECRET")
    cert.write_text("PUBLIC CERT")
    observed = {}

    def fake_run(command, capture_output, text, env):
        observed["env"] = env
        manifest_path = command[command.index("-m") + 1]
        output_path = command[command.index("-o") + 1]
        observed["manifest"] = json.loads(open(manifest_path, encoding="utf-8").read())
        shutil.copy2(source, output_path)
        return subprocess.CompletedProcess(command, 0, "ok", "")

    monkeypatch.setenv("C2PA_PRIVATE_KEY", "inherited secret")
    monkeypatch.setenv("C2PA_SIGN_CERT", "inherited cert")
    monkeypatch.setattr(sign.subprocess, "run", fake_run)

    ok, error = sign.sign_file(
        source, sign.manifest(title="Test", digital_source="digital"), str(key), str(cert),
        "/fake/c2patool"
    )

    assert ok, error
    assert "C2PA_PRIVATE_KEY" not in observed["env"]
    assert "C2PA_SIGN_CERT" not in observed["env"]
    assert observed["manifest"]["private_key"] == str(key)
    assert observed["manifest"]["sign_cert"] == str(cert)
    assert "PRIVATE SECRET" not in json.dumps(observed["manifest"])
    assert not [name for name in os.listdir(tmp_path) if ".signing-" in name]


@C2PATOOL
@OPENSSL
def test_real_sign_and_verify_round_trip_without_false_camera_claim(tmp_path):
    source = tmp_path / "signed.jpg"
    key = tmp_path / "keys" / "key.pem"
    cert = tmp_path / "keys" / "cert.pem"
    make_jpeg(source)
    assert sign.ensure_cert(key, cert, "Atelier Étoile")
    manifest = sign.manifest(title="Signed work", creator="José Ángel", org="Atelier Étoile",
                             digital_source="digital")

    ok, error = sign.sign_file(source, manifest, str(key), str(cert), sign.c2patool())

    assert ok, error
    report = core.inspect(source)
    assert report["c2pa_status"] == "present"
    assert report["c2pa"]["validation_state"] == "Valid"
    active = report["c2pa"]["active_manifest"]
    issuer = report["c2pa"]["manifests"][active]["signature_info"]["issuer"]
    assert issuer == "Atelier Étoile"
    assert report["ai"] is None


def test_sign_run_returns_failure_when_any_file_cannot_be_signed(tmp_path, monkeypatch):
    source = tmp_path / "source.jpg"
    make_jpeg(source)
    key = tmp_path / "key.pem"
    cert = tmp_path / "cert.pem"
    key.write_text("key")
    cert.write_text("cert")
    monkeypatch.setattr(sign, "c2patool", lambda: "/fake/c2patool")
    monkeypatch.setattr(sign, "sign_file", lambda *_args: (False, "deliberate failure"))
    args = argparse.Namespace(
        files=[str(source)], key=str(key), cert=str(cert), org="Studio", title=None,
        description=None, creator="Artist", url=None, contact=None, licence=None,
        generated=False, digital_source="digital",
    )

    assert sign.run(args) == 1
