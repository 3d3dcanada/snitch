# imprint

**Put your name on your work, and find out what a platform is about to remove.**

Your photo already carries a lot: what camera took it, when, often exactly where. What it usually
does not carry is who made it. And when you upload it, most platforms throw away the half you would
have wanted to keep.

```
$ imprint inspect holiday.jpg

holiday.jpg  3,204,118 bytes
  LOCATION IS IN THIS FILE
    GPSLatitude      45.9636
    GPSLongitude     -66.6431
    Anyone who downloads this can see where it was taken.
    Remove it:  imprint strip holiday.jpg
  camera
    Camera           Google Pixel 9
    Taken            2026:07:14 18:22:41
  NO CREDIT AT ALL
    Nothing in this file says who made it.
```

Three commands, three jobs:

| | |
|---|---|
| `imprint inspect` | what is in this image, including location and whether it says an AI made it |
| `imprint imprint` | write your credit in, and optionally burn it into the pixels |
| `imprint strip` | take metadata out, losslessly |

---

## Install

```bash
pip install imprint-cli
```

Needs [ExifTool](https://exiftool.org) for reading and writing metadata:

```bash
sudo apt install libimage-exiftool-perl     # Debian, Ubuntu
brew install exiftool                       # macOS
```

Signing C2PA Content Credentials additionally needs
[c2patool](https://github.com/contentauth/c2pa-rs): `cargo install c2patool`. Everything else works
without it.

---

## The part nobody tells you

```
$ imprint platforms

                 Visible stamp   C2PA Credentials   IPTC / XMP   EXIF
  LinkedIn       keeps           keeps              STRIPS       STRIPS
  Instagram      keeps           partial            STRIPS       STRIPS
  Facebook       keeps           partial            partial      STRIPS
  X / Twitter    keeps           partial            STRIPS       STRIPS
  Reddit         keeps           STRIPS             STRIPS       STRIPS
  Google Images  keeps           partial            keeps        keeps
```

**Only two things reliably survive a trip through a social platform: the pixels, and a C2PA
manifest on LinkedIn.** If credit matters to you, put it in the pixels.

`imprint platforms --notes` gives the detail on every cell. `--check` tells you how to verify any
row yourself in about two minutes, because this table is only useful if it is true. If a row is
wrong, please open an issue.

---

## Usage

### Find out what is in a file

```bash
imprint inspect photo.jpg
imprint snitch photo.jpg          # same thing
```

Reports GPS loudly, then camera, then credit, then any C2PA Content Credential, including whether
that credential says the image came from a generative model.

### Put your name on it

```bash
imprint imprint photo.jpg \
  --creator "Jane Doe" \
  --credit "Doe Studio" \
  --copyright "© 2026 Doe Studio" \
  --licence cc-by-nc \
  --url https://example.com \
  --contact hello@example.com
```

Writes the IPTC Core and XMP fields that picture desks and Google actually read: Creator, Credit,
CopyrightNotice, UsageTerms, WebStatement, LicensorName, LicensorURL, and keywords. **Strips GPS by
default**, because a credit line should not come with your home address. Pass `--keep-gps` if you
want it.

Licence presets: `cc-by`, `cc-by-sa`, `cc-by-nd`, `cc-by-nc`, `cc-by-nc-sa`, `cc-by-nc-nd`, `cc0`,
`arr`.

Add `--stamp-text "Doe Studio" --logo logo.png` to burn a visible mark into the corner at the same
time. That is the layer that survives everything.

### Take it back out

```bash
imprint strip photo.jpg
imprint no-comment photo.jpg      # same thing
```

Drops every JPEG `APPn`/`COM` segment, or every non-essential PNG chunk. **This is byte surgery,
not re-encoding**, so the decoded pixels come out identical and the tool proves it:

```
photo-clean.jpg  removed 24,118 bytes of metadata  pixels byte-identical
```

### Sign it so LinkedIn shows a credit badge

```bash
imprint sign photo.jpg --creator "Jane Doe" --org "Doe Studio" \
  --url https://example.com --licence cc-by-nc
imprint verify photo.jpg
```

LinkedIn scans uploads for a C2PA manifest and shows a "CR" badge on images that have one, opening
a panel that names the creator. It is currently the only major network that displays inbound
credentials rather than only reading them.

A key and certificate are generated on first use in `~/.config/imprint/`. Read the limits below
before relying on this.

---

## What this tool will not claim

Three things get overclaimed constantly in this space. None of them are true here.

**It does not remove invisible watermarks.** In-pixel watermarks such as Google SynthID are part of
the image data. They survive re-encoding, cropping and resizing by design. The only technique that
touches them is diffusion regeneration, which repaints the image; the tools that do it state that
they cannot detect whether it worked, and output remains classifiable as having been through a
removal pipeline. DeepMind reports 99.72% SynthID detection under worst-case transforms. `imprint
strip` removes **metadata**, including C2PA manifests. That is all it removes, and that is all it
says it removes.

**A self-signed credential does not prove who signed it.** The certificate generated on first use
produces a valid, tamper-evident manifest, and any validator will confirm the file has not changed
since signing. It is not on the C2PA trust list, so a validator that checks issuers reports the
signer as unknown rather than as you. Being on that list means buying a certificate from a CA in
the C2PA programme. Nothing here shortcuts that.

**Metadata does not create copyright.** You hold copyright in your work whether or not a file says
so. What metadata does is evidence authorship, carry your licence terms, and give an honest person
a way to credit you. That is genuinely useful, and it is a smaller claim than the one usually made.

---

## Why

Built by [3D3D](https://3d3d.ca) after spending a night putting provenance on a product and its
photographs and discovering that no single tool did the whole job, and that the tools which claimed
to do the hard part were not doing it.

The name is the double meaning: an imprint is a mark made by pressure, and it is the publisher's
name on a work.

MIT licensed. Issues and corrections welcome, particularly on the platform table.
