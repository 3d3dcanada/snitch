"""C2PA Content Credentials: signing.

WHY THIS IS WORTH DOING AT ALL. A C2PA manifest carries tamper-evident provenance in the file.
LinkedIn documents a gradually deployed icon and metadata panel for inbound C2PA content, and Meta
and Google document reading some C2PA signals. Their handling varies by product and upload path.
LinkedIn does not document whether an untrusted self-signed credential from this tool gets its UI.

THE HONEST LIMIT, STATED HERE AND IN THE OUTPUT. A certificate this tool generates for you is
SELF-SIGNED. That is enough for a readable, tamper-evident development credential whose asset
binding can validate. It is NOT a conforming identity credential and is not on the C2PA trust
list, so a validator that checks issuers reports the signer as unknown rather than as you. Being
on that list means obtaining a certificate from a CA in the C2PA programme. Nothing here can
shortcut that, and a tool that implied otherwise would be lying to you.
"""

import contextlib
import json
import os
import shutil
import subprocess
import sys
import tempfile

from . import __version__
from .core import LICENCES, _run


def config_dir(os_name=None, platform_name=None, environ=None, home=None):
    os_name = os_name or os.name
    platform_name = platform_name or sys.platform
    environ = os.environ if environ is None else environ
    home = os.path.expanduser("~") if home is None else home
    if os_name == "nt":
        root = environ.get("APPDATA") or os.path.join(home, "AppData", "Roaming")
        return os.path.join(root, "snitch")
    if platform_name == "darwin":
        return os.path.join(home, "Library", "Application Support", "snitch")
    root = environ.get("XDG_CONFIG_HOME") or os.path.join(home, ".config")
    return os.path.join(root, "snitch")


KEYDIR = config_dir()
DEFAULT_KEY = os.path.join(KEYDIR, "key.pem")
DEFAULT_CERT = os.path.join(KEYDIR, "cert.pem")

CAMERA = "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture"
GENERATED = "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia"
DIGITAL_SOURCES = {
    "camera": CAMERA,
    "digital": "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCreation",
    "screen": "http://cv.iptc.org/newscodes/digitalsourcetype/screenCapture",
    "human-edited": "http://cv.iptc.org/newscodes/digitalsourcetype/humanEdits",
    "generated": GENERATED,
    "ai-edited": (
        "http://cv.iptc.org/newscodes/digitalsourcetype/compositeWithTrainedAlgorithmicMedia"
    ),
    "algorithmic": "http://cv.iptc.org/newscodes/digitalsourcetype/algorithmicMedia",
    "data-driven": "http://cv.iptc.org/newscodes/digitalsourcetype/dataDrivenMedia",
}


def c2patool():
    from .core import resolve_c2patool
    return resolve_c2patool()


def _subject_value(value):
    if "\x00" in value or "\n" in value or "\r" in value:
        raise ValueError("certificate organisation cannot contain control characters")
    return value.replace("\\", "\\\\").replace("/", "\\/")


def _temporary_path(path):
    directory = os.path.dirname(os.path.abspath(path))
    os.makedirs(directory, mode=0o700, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=f".{os.path.basename(path)}.snitch-", dir=directory)
    os.close(fd)
    os.unlink(temporary)
    return temporary


def ensure_cert(key=DEFAULT_KEY, cert=DEFAULT_CERT, org="snitch"):
    """A P-256 signing certificate, made once.

    C2PA requires ES256 over prime256v1 for this algorithm, keyUsage digitalSignature and an
    extendedKeyUsage of emailProtection. A certificate without those is refused by the signer
    rather than producing a manifest that quietly fails to validate, which is the right way round.
    """
    if os.path.exists(key) and os.path.exists(cert):
        return False
    if os.path.exists(key) or os.path.exists(cert):
        present = key if os.path.exists(key) else cert
        missing = cert if os.path.exists(key) else key
        raise RuntimeError(f"refusing to replace existing {present}; matching file is missing: {missing}")
    if not shutil.which("openssl"):
        raise RuntimeError("openssl is not installed, and generating a signing key needs it")
    key_tmp = None
    cert_tmp = None
    try:
        key_tmp = _temporary_path(key)
        cert_tmp = _temporary_path(cert)
        r = _run(["openssl", "ecparam", "-name", "prime256v1", "-genkey", "-noout",
                  "-out", key_tmp])
        if r.returncode:
            raise RuntimeError(f"key generation failed: {r.stderr[:300]}")
        os.chmod(key_tmp, 0o600)
        subject_org = _subject_value(org)
        r = _run(["openssl", "req", "-new", "-x509", "-utf8", "-key", key_tmp,
                  "-out", cert_tmp, "-days", "3650", "-sha256",
                  "-subj", f"/CN={subject_org}/O={subject_org}",
                  "-addext", "basicConstraints=critical,CA:FALSE",
                  "-addext", "keyUsage=critical,digitalSignature",
                  "-addext", "extendedKeyUsage=critical,emailProtection"])
        if r.returncode:
            raise RuntimeError(f"certificate generation failed: {r.stderr[:300]}")
        os.replace(key_tmp, key)
        os.replace(cert_tmp, cert)
        return True
    finally:
        for temporary in (key_tmp, cert_tmp):
            if not temporary:
                continue
            with contextlib.suppress(FileNotFoundError):
                os.unlink(temporary)


def manifest(*, title, description=None, creator=None, org=None, url=None, contact=None,
             licence=None, digital_source=None, generated=False):
    work = {"@context": "https://schema.org", "@type": "CreativeWork", "name": title}
    if description:
        work["description"] = description
    if creator:
        work["author"] = [{"@type": "Person", "name": creator}]
    if org:
        work["creator"] = [{"@type": "Organization", "name": org}]
        work["copyrightHolder"] = [{"@type": "Organization", "name": org}]
        work["creditText"] = f"{org}{f' ({url})' if url else ''}"
    if url:
        work["url"] = url
    if contact:
        work.setdefault("creator", [{}])[0]["email"] = contact
    if licence and licence in LICENCES:
        name, lurl = LICENCES[licence]
        if lurl:
            work["license"] = lurl
        work["usageInfo"] = name

    if generated:
        digital_source = "generated"
    if digital_source not in DIGITAL_SOURCES:
        raise ValueError("a valid digital source is required for a C2PA created action")
    action: dict[str, object] = {
        "action": "c2pa.created",
        "digitalSourceType": DIGITAL_SOURCES[digital_source],
    }
    action["softwareAgent"] = {"name": "snitch", "version": __version__}

    return {
        "claim_generator_info": [{"name": "snitch", "version": __version__}],
        "title": title,
        "assertions": [
            {"label": "stds.schema-org.CreativeWork", "data": work},
            {"label": "c2pa.actions", "data": {"actions": [action]}},
        ],
    }


def sign_file(path, man, key, cert, tool):
    directory = os.path.dirname(os.path.abspath(path))
    stem, ext = os.path.splitext(os.path.basename(path))
    manifest_fd, mpath = tempfile.mkstemp(prefix=f".{stem}.snitch-", suffix=".json",
                                          dir=directory, text=True)
    output_fd, tmp = tempfile.mkstemp(prefix=f".{stem}.signing-", suffix=ext, dir=directory)
    os.close(output_fd)
    env = dict(os.environ)
    env.pop("C2PA_PRIVATE_KEY", None)
    env.pop("C2PA_SIGN_CERT", None)
    signing_manifest = dict(man)
    signing_manifest.update({"alg": "es256", "private_key": os.path.abspath(key),
                             "sign_cert": os.path.abspath(cert)})
    try:
        with os.fdopen(manifest_fd, "w", encoding="utf-8") as manifest_file:
            json.dump(signing_manifest, manifest_file, indent=2)
        r = subprocess.run([tool, path, "-m", mpath, "-o", tmp, "-f"],
                           capture_output=True, text=True, env=env, check=False)
        if r.returncode or not os.path.exists(tmp) or os.path.getsize(tmp) == 0:
            return False, (r.stderr or r.stdout)[:400]
        shutil.copystat(path, tmp)
        os.replace(tmp, path)
        return True, ""
    finally:
        for temporary in (mpath, tmp):
            with contextlib.suppress(FileNotFoundError):
                os.unlink(temporary)


def run(a):
    tool = c2patool()
    if not tool:
        print("\nc2patool is not installed, and signing needs it.\n"
              "  cargo install c2patool\n"
              "  or see https://github.com/contentauth/c2pa-rs\n")
        return 3

    key = a.key or DEFAULT_KEY
    cert = a.cert or DEFAULT_CERT
    try:
        made = ensure_cert(key, cert, a.org or "snitch")
    except (OSError, RuntimeError, ValueError) as exc:
        print(f"  certificate setup failed: {exc}")
        return 1
    if made:
        print(f"  generated a signing certificate at {cert}")
        print(f"  private key {key} (chmod 600, keep it)")

    failed = False
    signed = False
    for path in a.files:
        if not os.path.exists(path):
            print(f"  {path}: not found")
            failed = True
            continue
        man = manifest(title=a.title or os.path.basename(path), description=a.description,
                       creator=a.creator, org=a.org, url=a.url, contact=a.contact,
                       licence=a.licence, digital_source=a.digital_source,
                       generated=a.generated)
        ok, err = sign_file(path, man, key, cert, tool)
        print(f"  {os.path.basename(path)}  "
              f"{'signed' if ok else 'SIGN FAILED: ' + err}")
        signed = signed or ok
        failed = failed or not ok

    if signed:
        print("\n  This is a development-grade self-signed credential. Its asset binding is")
        print("  tamper-evident, but it is not a conforming identity credential or on the")
        print("  C2PA trust list. Strict validators report the signer as unknown rather than you.")
        print("  Getting on that list requires a certificate from a CA in the C2PA programme.")
    return 1 if failed else 0
