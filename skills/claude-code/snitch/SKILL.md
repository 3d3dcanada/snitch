---
name: snitch
description: Find out what an image is telling people, strip it, or put creator credit on it. Use WHENEVER the task involves image metadata, photo credit, copyright or attribution on pictures, EXIF or GPS in photos, IPTC/XMP fields, C2PA Content Credentials, watermarking or stamping a logo onto images, checking whether an image was AI-generated, or preparing photographs for upload to a social platform, a stock site or a model repository. Also use when someone asks "will my copyright survive if I post this", "does this photo have my location in it", or "how do I stop people using my images without credit".
---

# snitch, no-comment, credit

Three commands, one job each. Install: `pip install snitch-tools` plus ExifTool.
Source: https://github.com/3d3dcanada/snitch

| | |
|---|---|
| `snitch FILE` | what the file is telling people: GPS, camera, credit, C2PA, AI provenance |
| `snitch --platforms` | sourced platform metadata-handling research, with unknowns marked |
| `no-comment FILE` | strip metadata, losslessly, proving the pixels do not change |
| `credit FILE --creator ...` | write credit in, `--stamp` into the pixels, `--sign` a C2PA credential |

## Start here, always

Run `snitch` on the file **before** doing anything else, and read the result out. Two findings
change what the user actually wants:

- **GPS present.** Say so plainly and early. A holiday photo with coordinates in it is the user's
  home address, and most people have no idea it is there.
- **No credit at all.** The normal state of a camera file, and the thing they are about to lose by
  uploading.

## The rule that decides the advice

**Pixels are the only broadly portable layer, and even they can be cropped or softened.** Platform
handling of IPTC/XMP and C2PA varies by route. LinkedIn documents C2PA display, but rollout and
untrusted self-signed handling are not verified here.

So when someone asks how to make sure they get credit:

1. **`credit --stamp` is the most portable option** if credit matters. It survives metadata
   stripping and screenshots, but a crop can still remove it.
2. **`credit --sign --digital-source SOURCE`** adds a development-grade, tamper-evident credential.
   It does not add a current CAWG identity assertion. Do not promise that a platform will display
   it; test the exact upload route.
3. **The IPTC/XMP fields** are worth writing. Google Images documents reading specific rights and
   source fields, and established publishing workflows use them.

Run `snitch --platforms` and show the table rather than paraphrasing it.

## Worked example

```bash
snitch shot.jpg

credit shot.jpg \
  --creator "Jane Doe" --credit "Doe Studio" \
  --copyright "© 2026 Doe Studio" --licence cc-by-nc \
  --url https://example.com --contact hello@example.com \
  --stamp "Doe Studio" --stamp-sub "example.com" --logo logo.png \
  --sign --digital-source camera

snitch shot-credited.jpg
```

Licence presets: `cc-by`, `cc-by-sa`, `cc-by-nd`, `cc-by-nc`, `cc-by-nc-sa`, `cc-by-nc-nd`, `cc0`,
`arr`. `credit` **strips GPS by default**; keep it only if the user asks and means it.

## Three things never to tell the user

1. **`no-comment` does not remove invisible watermarks.** It removes metadata, including C2PA
   manifests. In-pixel watermarks such as Google SynthID are part of the image data. Metadata
   stripping does not touch them; removal attempts repaint pixels and cannot prove the mark is
   gone.
2. **A self-signed credential does not prove who signed it.** Its asset binding is tamper-evident,
   but it is a development credential with a legacy creator metadata assertion, not a current CAWG
   identity assertion. It is not on the C2PA trust list, so strict validators report the signer as
   unknown. A conforming claim generator needs an eligible certificate from a CA in the C2PA
   programme.
3. **Metadata does not create copyright.** Copyright exists without it. Metadata evidences
   authorship and carries licence terms. Say the smaller, true thing.

## When someone wants metadata removed

Legitimate and common: people strip GPS before posting, photographers strip client data before
delivery. `no-comment` is lossless and proves the pixels are byte-identical. Report that proof, it
is the reassuring part.

If the intent is clearly to remove someone else's credit from work that is not theirs, say so once
and do not do it.
