# snitch

**Your photo is telling on you. Find out what, make it stop, or put your name on it instead.**

Three tools. You type the name of the thing you want.

```
snitch      photo.jpg    what is this file telling people about you
no-comment  photo.jpg    make it stop
credit      photo.jpg    put your name on it, so it stays
```

---

## snitch

```
$ snitch holiday.jpg

holiday.jpg  3,204,118 bytes
  LOCATION IS IN THIS FILE
    GPSLatitude      45.9636
    GPSLongitude     -66.6431
    Anyone who downloads this can see where it was taken.
    Make it stop:  no-comment holiday.jpg
  camera
    Camera           Google Pixel 9
    Taken            2026:07:14 18:22:41
  NO CREDIT AT ALL
    Nothing in this file says who made it.
    Fix it:  credit holiday.jpg --creator "Your Name"
  no C2PA Content Credential
```

It reads EXIF, IPTC, XMP and C2PA Content Credentials, and tells you whether a credential says the
image came from a generative model.

### The part nobody tells you

```
$ snitch --platforms

                 Visible stamp   C2PA Credentials   IPTC / XMP   EXIF
  LinkedIn       ? keeps         D partial           ? unknown    ? unknown
  Instagram      ? keeps         D partial           D partial    ? unknown
  Facebook       ? keeps         D partial           D partial    ? unknown
  X / Twitter    ? keeps         ? unknown           ? unknown    D STRIPS
  Reddit         ? keeps         ? unknown           ? unknown    D partial
  Printables     ? keeps         ? unknown           ? unknown    ? unknown
  Google Images  ? keeps         D partial           D reads      ? unknown
```

`D` means the platform documents the behaviour. `C` means independent upload/download tests
corroborate it. `?` means it is an explicitly unverified expectation, not a fact.

**Pixels are the only broadly portable layer.** LinkedIn documents C2PA display, but the rollout is
gradual and its handling of this tool's untrusted self-signed credentials has not been live-tested.
If credit matters, put it in the pixels and test your exact upload route.

`--notes` gives each cell's evidence class, limitation, and source URL. `--check` gives a repeatable
before/upload/download/after procedure. Platform behaviour can differ by feed, story, ad, message,
client, account, and file type, so corrections should include that context.

---

## no-comment

```
$ no-comment holiday.jpg
  holiday-clean.jpg  removed 24,118 bytes of metadata  pixels byte-identical
```

Drops private/application metadata while retaining JPEG JFIF, ICC colour, Adobe colour-transform,
and orientation data, plus PNG colour, transparency, orientation, and animation chunks. **This is
byte surgery, not re-encoding**, so every decoded frame comes out identical and the tool proves it
rather than asking you to take its word.

`--in-place` atomically replaces the input. `--out DIR` handles batches; existing outputs require
`--force`. In-place symlinks are refused rather than silently replacing the link.

---

## credit

```
$ credit shot.jpg \
    --creator "Jane Doe" \
    --credit "Doe Studio" \
    --copyright "© 2026 Doe Studio" \
    --licence cc-by-nc \
    --url https://example.com \
    --contact hello@example.com \
    --stamp "Doe Studio" --stamp-sub "example.com" --logo logo.png \
    --sign --digital-source camera
```

Writes the IPTC Core and XMP fields that picture desks and Google actually read: Creator, Credit,
CopyrightNotice, UsageTerms, WebStatement, LicensorName, LicensorURL, keywords.

**Strips GPS by default**, because a credit line should not come with your home address. Pass
`--keep-gps` if you want it.

- `--stamp` burns a visible mark into the pixels. **That is the only layer that survives
  screenshots**, though a platform can still crop or soften it. Stamping currently supports JPEG
  and PNG; it preserves existing PNG transparency.
- `--sign --digital-source camera` adds a development-grade self-signed C2PA Content Credential
  and requires an explicit source type so it never guesses camera versus AI provenance. It does
  not add a current CAWG identity assertion. LinkedIn documents inbound C2PA display, but rollout
  and untrusted self-signed handling remain unverified here.
- `--verify` checks an existing credential instead of writing anything.

Metadata-only credit has been exercised on JPEG, PNG, WebP, TIFF, and HEIC. Exact namespaces vary
by container and ExifTool support. Stamping is deliberately refused for WebP, TIFF, and HEIC rather
than writing a different format under the old extension.

`snitch --json FILE` emits a stable machine-readable report for scripts and batch checks.

Licence presets: `cc-by`, `cc-by-sa`, `cc-by-nd`, `cc-by-nc`, `cc-by-nc-sa`, `cc-by-nc-nd`, `cc0`,
`arr`.

---

## Install

```bash
pip install snitch-tools
```

Needs [ExifTool](https://exiftool.org):

```bash
sudo apt install libimage-exiftool-perl     # Debian, Ubuntu
brew install exiftool                       # macOS
```

Signing and full C2PA validation additionally need
[c2patool](https://github.com/contentauth/c2pa-rs): `cargo install c2patool`. Without it, `snitch`
reports C2PA validation as unavailable instead of falsely reporting that no credential exists;
ExifTool can still detect a C2PA/JUMBF container.

### Development checks

```bash
python -m pip install -e '.[dev]'
ruff check snitch tests
mypy snitch
pytest -q
python -m build
python -m twine check dist/*
```

CI runs those checks on Linux and macOS, exercises the declared Python 3.9 floor and a current
Python, and also runs the command suite on Windows.

### Agent skills

Drop-in skills for Claude Code and Kilo Code are in [`skills/`](skills/). One `cp` each.

---

## What these tools will not claim

Three things get overclaimed constantly in this space. None of them are true here.

**`no-comment` does not remove invisible watermarks.** In-pixel watermarks such as Google SynthID
are part of the image data. They survive re-encoding, cropping and resizing by design. The only
technique that touches them is diffusion regeneration, which repaints the image; the tools that do
it state they cannot detect whether it worked, and output remains classifiable as having been
through a removal pipeline. A 2025
[SynthID-Image evaluation](https://arxiv.org/abs/2510.09263) reported 99.72% true-positive
detection at 0.1% false positives for its external SynthID-O variant at its preferred resolution,
aggregated across the study's worst transformation settings.
`no-comment` removes **metadata**, including C2PA manifests. That is all it removes, and all it
says it removes.

**A self-signed credential does not prove who signed it.** The certificate generated on first use
produces a readable, tamper-evident development credential whose asset binding can validate. Its
creator block is a legacy metadata assertion, not a current CAWG identity assertion, and the
certificate is not on the C2PA trust list. A validator that checks identity therefore reports the
signer as unknown rather than as you. A conforming claim generator needs an eligible certificate
from a CA in the C2PA programme. Nothing here shortcuts that.

**Metadata does not create copyright.** You hold copyright in your work whether or not a file says
so. What metadata does is evidence authorship, carry your licence terms, and give an honest person
a way to credit you. That is genuinely useful, and it is a smaller claim than the one usually made.

---

## Why

Built by [3D3D](https://3d3d.ca) after a night spent putting provenance on a product and its
photographs, and finding that no single tool did the whole job, and that the tools claiming to do
the hard part were not doing it.

MIT licensed. Issues and corrections welcome, particularly on the platform table.
