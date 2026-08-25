"""C2PA Content Credentials: signing.

WHY THIS IS WORTH DOING AT ALL. LinkedIn scans every uploaded image for a C2PA manifest and, when
it finds one, shows a "CR" badge on the image that opens a panel naming the creator and the tool.
As of 2026 it is the only major network that DISPLAYS inbound credentials rather than only reading
them to decide whether to apply an AI label. IPTC and XMP, which `credit` writes, are
stripped by LinkedIn on upload. This is not.

THE HONEST LIMIT, STATED HERE AND IN THE OUTPUT. A certificate this tool generates for you is
SELF-SIGNED. That is enough for a valid, readable, tamper-evident manifest, and any validator will
confirm the file has not been altered since you signed it. It is NOT on the C2PA trust list, so a
validator that checks issuers reports the signer as unknown rather than as you. Being on that list
means buying a certificate from a CA in the C2PA programme. Nothing here can shortcut that, and a
tool that implied otherwise would be lying to you.
"""

import json
import os
import shutil
import subprocess

from .core import LICENCES, _run

KEYDIR = os.path.expanduser("~/.config/snitch")
DEFAULT_KEY = os.path.join(KEYDIR, "key.pem")
DEFAULT_CERT = os.path.join(KEYDIR, "cert.pem")

CAMERA = "http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture"
GENERATED = "http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia"


def c2patool():
    return (shutil.which("c2patool")
            or (os.path.expanduser("~/.cargo/bin/c2patool")
                if os.path.exists(os.path.expanduser("~/.cargo/bin/c2patool")) else None))


def ensure_cert(key=DEFAULT_KEY, cert=DEFAULT_CERT, org="snitch"):
    """A P-256 signing certificate, made once.

    C2PA requires ES256 over prime256v1 for this algorithm, keyUsage digitalSignature and an
    extendedKeyUsage of emailProtection. A certificate without those is refused by the signer
    rather than producing a manifest that quietly fails to validate, which is the right way round.
    """
    if os.path.exists(key) and os.path.exists(cert):
        return False
    if not shutil.which("openssl"):
        raise RuntimeError("openssl is not installed, and generating a signing key needs it")
    os.makedirs(os.path.dirname(key), exist_ok=True)
    os.chmod(os.path.dirname(key), 0o700)
    r = _run(["openssl", "ecparam", "-name", "prime256v1", "-genkey", "-noout", "-out", key])
    if r.returncode:
        raise RuntimeError(f"key generation failed: {r.stderr[:300]}")
    os.chmod(key, 0o600)
    r = _run(["openssl", "req", "-new", "-x509", "-key", key, "-out", cert,
              "-days", "3650", "-sha256", "-subj", f"/CN={org}/O={org}",
              "-addext", "basicConstraints=critical,CA:FALSE",
              "-addext", "keyUsage=critical,digitalSignature",
              "-addext", "extendedKeyUsage=critical,emailProtection"])
    if r.returncode:
        raise RuntimeError(f"certificate generation failed: {r.stderr[:300]}")
    return True


def manifest(*, title, description=None, creator=None, org=None, url=None, contact=None,
             licence=None, generated=False):
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

    return {
        "claim_generator_info": [{"name": org or "snitch", "version": "1.0"}],
        "title": title,
        "assertions": [
            {"label": "stds.schema-org.CreativeWork", "data": work},
            {"label": "c2pa.actions", "data": {"actions": [{
                "action": "c2pa.created",
                "digitalSourceType": GENERATED if generated else CAMERA,
                **({"softwareAgent": {"name": org}} if org else {}),
            }]}},
        ],
    }


def sign_file(path, man, key, cert, tool):
    mpath = path + ".manifest.json"
    with open(mpath, "w", encoding="utf-8") as f:
        json.dump(man, f, indent=2)
    # THE TEMP FILE MUST KEEP THE SOURCE EXTENSION. c2patool refuses with "Output type must
    # match source type" otherwise, because it picks the container format off the filename.
    stem, ext = os.path.splitext(path)
    tmp = f"{stem}.signing{ext}"
    env = dict(os.environ)
    env["C2PA_PRIVATE_KEY"] = open(key).read()
    env["C2PA_SIGN_CERT"] = open(cert).read()
    r = subprocess.run([tool, path, "-m", mpath, "-o", tmp, "-f"],
                       capture_output=True, text=True, env=env)
    os.remove(mpath)
    if r.returncode or not os.path.exists(tmp):
        return False, (r.stderr or r.stdout)[:400]
    shutil.move(tmp, path)
    return True, ""


def run(a):
    tool = c2patool()
    if not tool:
        print("\nc2patool is not installed, and signing needs it.\n"
              "  cargo install c2patool\n"
              "  or see https://github.com/contentauth/c2pa-rs\n")
        return 3

    key = a.key or DEFAULT_KEY
    cert = a.cert or DEFAULT_CERT
    made = ensure_cert(key, cert, a.org or "snitch")
    if made:
        print(f"  generated a signing certificate at {cert}")
        print(f"  private key {key} (chmod 600, keep it)")

    for path in a.files:
        if not os.path.exists(path):
            print(f"  {path}: not found")
            continue
        man = manifest(title=a.title or os.path.basename(path), description=a.description,
                       creator=a.creator, org=a.org, url=a.url, contact=a.contact,
                       licence=a.licence, generated=a.generated)
        ok, err = sign_file(path, man, key, cert, tool)
        print(f"  {os.path.basename(path)}  "
              f"{'signed' if ok else 'SIGN FAILED: ' + err}")

    print("\n  This certificate is self-signed. The credential is valid and tamper-evident,")
    print("  and proves the file has not changed since you signed it. It is not on the C2PA")
    print("  trust list, so strict validators will report the signer as unknown rather than")
    print("  as you. Getting on that list means buying a certificate from a CA in the C2PA")
    print("  programme.")
    return 0
