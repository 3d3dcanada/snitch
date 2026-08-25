---
name: imprint
description: Put creator credit into images and find out what a platform will strip. Use WHENEVER the task involves image metadata, photo credit, copyright or attribution on pictures, EXIF or GPS in photos, IPTC/XMP fields, C2PA Content Credentials, watermarking or stamping a logo onto images, checking whether an image was AI-generated, or preparing photographs for upload to a social platform, a stock site or a model repository. Also use when someone asks "will my copyright survive if I post this", "does this photo have my location in it", or "how do I stop people using my images without credit".
---

# imprint

Three jobs people usually conflate, and the difference matters:

| | |
|---|---|
| `imprint inspect FILE` | what is in this image: GPS, camera, credit, C2PA, AI provenance |
| `imprint imprint FILE --creator ...` | write credit in, optionally stamp it into the pixels |
| `imprint strip FILE` | take metadata out, losslessly |
| `imprint sign FILE --creator ...` | add a C2PA Content Credential |
| `imprint platforms` | what each platform keeps and strips |

Install: `pip install imprint-cli`, plus ExifTool. Source: https://github.com/3d3dcanada/imprint

## Start here, always

Run `imprint inspect` on the file **before** doing anything else, and read it out. Two findings
change what the user wants:

- **GPS present.** Say so plainly and early. A holiday photo with coordinates in it is the user's
  home address, and most people have no idea it is there.
- **No credit at all.** This is the normal state of a camera file and it is the thing the user is
  usually about to lose by uploading.

## The rule that decides the advice

**Only two layers reliably survive a social platform: the pixels, and a C2PA manifest on
LinkedIn.** IPTC and XMP are stripped by LinkedIn, Instagram, X, Reddit and most model
repositories.

So when someone asks "how do I make sure I get credit":

1. **Visible stamp** is not optional if credit actually matters. It is the only layer that survives
   a screenshot, a re-encode and a repost.
2. **C2PA** is worth doing if LinkedIn is in the plan, because LinkedIn displays a CR badge naming
   the creator.
3. **IPTC/XMP** is worth doing anyway. Google Images reads it for the Licensable badge, picture
   desks respect it, and it costs nothing.

Run `imprint platforms` and show the table rather than paraphrasing it.

## Worked example

```bash
imprint inspect shot.jpg

imprint imprint shot.jpg \
  --creator "Jane Doe" --credit "Doe Studio" \
  --copyright "© 2026 Doe Studio" --licence cc-by-nc \
  --url https://example.com --contact hello@example.com \
  --stamp-text "Doe Studio" --stamp-sub "example.com" --logo logo.png

imprint sign shot.jpg --creator "Jane Doe" --org "Doe Studio" --licence cc-by-nc
imprint verify shot.jpg
```

Licence presets: `cc-by`, `cc-by-sa`, `cc-by-nd`, `cc-by-nc`, `cc-by-nc-sa`, `cc-by-nc-nd`, `cc0`,
`arr`.

`imprint imprint` **strips GPS by default**. Keep it only if the user asks, and confirm they mean
it.

## Three things never to tell the user

These get overclaimed constantly. Do not repeat them.

1. **It does not remove invisible watermarks.** `imprint strip` removes metadata, including C2PA
   manifests. In-pixel watermarks such as Google SynthID are part of the image data, survive
   re-encoding and cropping by design, and no tool removes them. If asked directly, say that
   plainly: the only technique that touches them repaints the image, cannot verify its own success,
   and leaves output still classifiable as having been through a removal pipeline.
2. **A self-signed credential does not prove who signed it.** The certificate `imprint sign`
   generates makes a valid, tamper-evident manifest, and it proves the file has not changed since
   signing. It is not on the C2PA trust list, so strict validators report the signer as unknown.
   Being on that list means buying a certificate from a CA in the C2PA programme.
3. **Metadata does not create copyright.** Copyright exists without it. Metadata evidences
   authorship and carries licence terms. Say the smaller, true thing.

## When someone wants metadata removed

That is a legitimate and common need: people strip GPS before posting, and photographers strip
client data before delivery. `imprint strip` does it losslessly and proves the pixels are
byte-identical. Report that proof, it is the reassuring part.

If the intent is clearly to remove someone else's credit from work that is not theirs, say so once
and do not do it.
