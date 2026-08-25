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
  LinkedIn       keeps           keeps              STRIPS       STRIPS
  Instagram      keeps           partial            STRIPS       STRIPS
  Facebook       keeps           partial            partial      STRIPS
  X / Twitter    keeps           partial            STRIPS       STRIPS
  Reddit         keeps           STRIPS             STRIPS       STRIPS
  Google Images  keeps           partial            keeps        keeps
```

**Only two things reliably survive a trip through a social platform: the pixels, and a C2PA
manifest on LinkedIn.** If credit matters to you, put it in the pixels.

`--notes` gives the detail on every cell. `--check` tells you how to verify any row yourself in
about two minutes, because this table is only useful if it is true. If a row is wrong, please open
an issue.

---

## no-comment

```
$ no-comment holiday.jpg
  holiday-clean.jpg  removed 24,118 bytes of metadata  pixels byte-identical
```

Drops every JPEG `APPn`/`COM` segment, or every non-essential PNG chunk. **This is byte surgery,
not re-encoding**, so the decoded pixels come out identical and the tool proves it rather than
asking you to take its word.

`--in-place` to overwrite instead of writing a copy.

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
    --sign
```

Writes the IPTC Core and XMP fields that picture desks and Google actually read: Creator, Credit,
CopyrightNotice, UsageTerms, WebStatement, LicensorName, LicensorURL, keywords.

**Strips GPS by default**, because a credit line should not come with your home address. Pass
`--keep-gps` if you want it.

- `--stamp` burns a visible mark into the pixels. **That is the only layer that survives
  everything**, and it is why the flag exists.
- `--sign` adds a C2PA Content Credential. LinkedIn scans uploads for one and shows a "CR" badge
  naming the creator. It is currently the only major network that displays inbound credentials
  rather than only reading them.
- `--verify` checks an existing credential instead of writing anything.

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

`credit --sign` additionally needs [c2patool](https://github.com/contentauth/c2pa-rs):
`cargo install c2patool`. Everything else works without it.

### Agent skills

Drop-in skills for Claude Code and Kilo Code are in [`skills/`](skills/). One `cp` each.

---

## What these tools will not claim

Three things get overclaimed constantly in this space. None of them are true here.

**`no-comment` does not remove invisible watermarks.** In-pixel watermarks such as Google SynthID
are part of the image data. They survive re-encoding, cropping and resizing by design. The only
technique that touches them is diffusion regeneration, which repaints the image; the tools that do
it state they cannot detect whether it worked, and output remains classifiable as having been
through a removal pipeline. DeepMind reports 99.72% SynthID detection under worst-case transforms.
`no-comment` removes **metadata**, including C2PA manifests. That is all it removes, and all it
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

Built by [3D3D](https://3d3d.ca) after a night spent putting provenance on a product and its
photographs, and finding that no single tool did the whole job, and that the tools claiming to do
the hard part were not doing it.

MIT licensed. Issues and corrections welcome, particularly on the platform table.
