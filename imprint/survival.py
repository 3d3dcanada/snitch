"""What each platform does to your metadata when you upload.

THIS TABLE IS THE POINT OF THE WHOLE TOOL. Writing credit into a file is easy; knowing which
half of it a platform is about to throw away is the part nobody tells you, and it is the
difference between "I added my copyright" and "my copyright is still there".

Last verified 2026-08-25. Re-verify before trusting it: platforms change this quietly and
without announcement. `imprint platforms --check` prints how to test any of them yourself in
about two minutes.
"""

KEEP = "keeps"
STRIP = "strips"
PARTIAL = "partial"
UNKNOWN = "unknown"

VERIFIED = "2026-08-25"

# platform -> {layer: (verdict, note)}
PLATFORMS = {
    "LinkedIn": {
        "exif": (STRIP, "Removed on upload, including GPS."),
        "iptc_xmp": (STRIP, "Removed on upload. Your Creator and Copyright fields do not survive."),
        "c2pa": (KEEP, "KEEPS IT, AND SHOWS IT. LinkedIn scans uploads for a C2PA manifest and "
                       "displays a 'CR' badge on the image. Clicking it names the creator and "
                       "the tool. As of 2026 it is the only major network that displays inbound "
                       "credentials rather than only reading them."),
        "visible": (KEEP, "A pixel is a pixel."),
    },
    "Instagram": {
        "exif": (STRIP, "Removed, including GPS."),
        "iptc_xmp": (STRIP, "Removed."),
        "c2pa": (PARTIAL, "Read on upload to decide whether to apply an 'AI info' label, but not "
                          "preserved for anyone who downloads the image."),
        "visible": (KEEP, "Survives, though heavy re-encoding can soften a small stamp."),
    },
    "Facebook": {
        "exif": (STRIP, "Removed, including GPS."),
        "iptc_xmp": (PARTIAL, "IPTC copyright fields are sometimes retained internally, but are "
                              "not served on the public file."),
        "c2pa": (PARTIAL, "Read for AI labelling, not preserved for download."),
        "visible": (KEEP, "Survives."),
    },
    "X / Twitter": {
        "exif": (STRIP, "Removed on upload."),
        "iptc_xmp": (STRIP, "Removed on upload."),
        "c2pa": (PARTIAL, "Inconsistent. Re-encoding usually destroys the manifest."),
        "visible": (KEEP, "Survives."),
    },
    "Reddit": {
        "exif": (STRIP, "Removed on upload."),
        "iptc_xmp": (STRIP, "Removed on upload."),
        "c2pa": (STRIP, "Re-encoded, manifest lost."),
        "visible": (KEEP, "Survives."),
    },
    "Printables": {
        "exif": (STRIP, "Re-encoded on upload."),
        "iptc_xmp": (STRIP, "Re-encoded on upload."),
        "c2pa": (UNKNOWN, "Not documented. Assume it is lost."),
        "visible": (KEEP, "Survives, and is the only credit that does."),
    },
    "Google Images": {
        "exif": (KEEP, "Read from the file on your own site."),
        "iptc_xmp": (KEEP, "READS IT. IPTC Creator, Credit and the rights fields drive the "
                           "'Licensable' badge and the image-licence panel in search results. "
                           "This is the single best reason to write IPTC properly."),
        "c2pa": (PARTIAL, "Surfaced in 'About this image' where present."),
        "visible": (KEEP, "Survives."),
    },
}

LAYERS = [
    ("visible", "Visible stamp", "Pixels. Survives screenshots, re-encoding, reposting."),
    ("c2pa", "C2PA Content Credentials", "Signed, tamper-evident provenance manifest."),
    ("iptc_xmp", "IPTC / XMP", "Creator, Credit, Copyright, Usage Terms, Licensor URL."),
    ("exif", "EXIF", "Camera, lens, date, and GPS."),
]

SYMBOL = {KEEP: "keeps", STRIP: "STRIPS", PARTIAL: "partial", UNKNOWN: "unknown"}


def summary_for(platform):
    return PLATFORMS.get(platform)


def one_line_advice():
    """The conclusion the table exists to support."""
    return (
        "Only two things reliably survive a trip through a social platform: the pixels, and "
        "a C2PA manifest on LinkedIn. If credit matters to you, put it in the pixels."
    )


def how_to_verify():
    return """Verify any row yourself in about two minutes:

  1. imprint imprint photo.jpg --creator "Your Name" --credit "Your Studio"
  2. Upload it to the platform.
  3. Download the image back off the platform, at full size.
  4. imprint inspect downloaded.jpg

Whatever is missing in step 4 is what that platform strips. Please open an issue if a row
here is wrong: this table is only useful if it is true."""
