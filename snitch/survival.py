"""Sourced claims about what platforms do to image metadata.

This is a research index, not a blanket verification claim. A platform can read metadata during
ingestion, retain it internally, show a label, and still omit it from the downloadable derivative.
Those behaviours are recorded separately in each note. No upload/download round trips were
performed for this 2026-08-25 review, so unsupported expectations are visibly marked unverified.
"""

KEEP = "keeps"
STRIP = "strips"
PARTIAL = "partial"
READS = "reads"
UNKNOWN = "unknown"

DOCUMENTED = "documented"
CORROBORATED = "corroborated"
INFERENCE = "inference"
EVIDENCE_CLASSES = (DOCUMENTED, CORROBORATED, INFERENCE)

RESEARCHED = "2026-08-25"

LINKEDIN_C2PA = {
    "title": "LinkedIn Help — Content credentials",
    "url": "https://www.linkedin.com/help/linkedin/answer/a6282984",
}
META_C2PA = {
    "title": "Meta — Labeling AI-Generated Images on Facebook, Instagram and Threads",
    "url": "https://about.fb.com/news/2024/02/labeling-ai-generated-images-on-facebook-instagram-and-threads/",
}
META_2026 = {
    "title": "Meta — How Meta Is Preparing for the 2026 US Midterm Elections",
    "url": "https://about.fb.com/news/2026/02/meta-prepares-for-2026-us-midterms/",
}
X_EXIF = {
    "title": "X Help — How to post photos or GIFs",
    "url": "https://help.x.com/en/using-x/posting-gifs-and-pictures",
}
REDDIT_PRIVACY = {
    "title": "Reddit — Privacy by design",
    "url": "https://redditinc.com/privacy",
}
GOOGLE_METADATA = {
    "title": "Google Search Central — Image metadata in Google Images",
    "url": (
        "https://developers.google.com/search/docs/appearance/structured-data/"
        "image-license-metadata"
    ),
}
GOOGLE_DETAILS = {
    "title": "Google Search Help — Find Google Image details",
    "url": "https://support.google.com/websearch/answer/9789430",
}


def cell(verdict, evidence, note, *sources):
    return {
        "verdict": verdict,
        "evidence": evidence,
        "note": note,
        "sources": list(sources),
    }


# Platform -> layer -> evidence record. An inference can still carry an expected verdict, but the
# renderer prefixes it with "?" so an expectation can never masquerade as a verified round trip.
PLATFORMS = {
    "LinkedIn": {
        "exif": cell(
            UNKNOWN, INFERENCE,
            "No current LinkedIn source found for EXIF in the public/downloadable derivative. "
            "The previous STRIPS claim was not live-tested.",
        ),
        "iptc_xmp": cell(
            UNKNOWN, INFERENCE,
            "No current LinkedIn source found for IPTC/XMP credit fields in the public/downloadable "
            "derivative. The previous STRIPS claim was not live-tested.",
        ),
        "c2pa": cell(
            PARTIAL, DOCUMENTED,
            "LinkedIn documents an icon and metadata panel for uploads containing C2PA data, but "
            "says rollout is gradual. It does not document whether this tool's untrusted self-signed "
            "credentials qualify or whether a downloaded derivative retains the manifest.",
            LINKEDIN_C2PA,
        ),
        "visible": cell(
            KEEP, INFERENCE,
            "Expected to remain in displayed pixels, subject to cropping and re-encoding. Not "
            "upload/download tested in this review.",
        ),
    },
    "Instagram": {
        "exif": cell(
            UNKNOWN, INFERENCE,
            "Meta documents collecting media metadata, but no current source found defines EXIF "
            "fields in Instagram's public/downloadable derivative.",
        ),
        "iptc_xmp": cell(
            PARTIAL, DOCUMENTED,
            "Meta documents reading IPTC AI-generation signals for labels. That does not establish "
            "that creator, copyright, or rights fields survive in a downloadable derivative.",
            META_C2PA,
        ),
        "c2pa": cell(
            PARTIAL, DOCUMENTED,
            "Meta documents using C2PA signals to identify and label AI content. It does not "
            "document preservation of the manifest in a downloadable Instagram derivative.",
            META_C2PA, META_2026,
        ),
        "visible": cell(
            KEEP, INFERENCE,
            "Expected to remain in displayed pixels, subject to cropping and re-encoding. Not "
            "upload/download tested in this review.",
        ),
    },
    "Facebook": {
        "exif": cell(
            UNKNOWN, INFERENCE,
            "Meta documents collecting media metadata, but no current source found defines EXIF "
            "fields in Facebook's public/downloadable derivative.",
        ),
        "iptc_xmp": cell(
            PARTIAL, DOCUMENTED,
            "Meta documents reading IPTC AI-generation signals for labels. That does not establish "
            "that creator, copyright, or rights fields survive in a downloadable derivative.",
            META_C2PA,
        ),
        "c2pa": cell(
            PARTIAL, DOCUMENTED,
            "Meta documents using C2PA signals to identify and label AI content. It does not "
            "document preservation of the manifest in a downloadable Facebook derivative.",
            META_C2PA, META_2026,
        ),
        "visible": cell(
            KEEP, INFERENCE,
            "Expected to remain in displayed pixels, subject to cropping and re-encoding. Not "
            "upload/download tested in this review.",
        ),
    },
    "X / Twitter": {
        "exif": cell(
            STRIP, DOCUMENTED,
            "X says it reads EXIF during upload, retains it temporarily for processing, and does "
            "not make it available to people viewing the posted photo.",
            X_EXIF,
        ),
        "iptc_xmp": cell(
            UNKNOWN, INFERENCE,
            "X's current help covers EXIF but not IPTC/XMP credit fields in the served derivative. "
            "The previous STRIPS claim was not live-tested.",
        ),
        "c2pa": cell(
            UNKNOWN, INFERENCE,
            "No current X source or live round trip was found for inbound C2PA manifests.",
        ),
        "visible": cell(
            KEEP, INFERENCE,
            "Expected to remain in displayed pixels, subject to automatic scaling, cropping, and "
            "re-encoding. Not upload/download tested in this review.",
        ),
    },
    "Reddit": {
        "exif": cell(
            PARTIAL, DOCUMENTED,
            "Reddit says it strips location and personal-information metadata from shared images. "
            "It does not say that every EXIF field is removed.",
            REDDIT_PRIVACY,
        ),
        "iptc_xmp": cell(
            UNKNOWN, INFERENCE,
            "Reddit's privacy statement is not field-specific enough to prove what happens to IPTC "
            "and XMP rights metadata.",
        ),
        "c2pa": cell(
            UNKNOWN, INFERENCE,
            "No current Reddit source or live round trip was found for C2PA preservation.",
        ),
        "visible": cell(
            KEEP, INFERENCE,
            "Expected to remain in displayed pixels, subject to cropping and re-encoding. Not "
            "upload/download tested in this review.",
        ),
    },
    "Printables": {
        "exif": cell(
            UNKNOWN, INFERENCE,
            "Printables does not document this and no account-based round trip was authorized.",
        ),
        "iptc_xmp": cell(
            UNKNOWN, INFERENCE,
            "Printables does not document this and no account-based round trip was authorized.",
        ),
        "c2pa": cell(
            UNKNOWN, INFERENCE,
            "Printables does not document inbound C2PA handling and no live round trip was run.",
        ),
        "visible": cell(
            KEEP, INFERENCE,
            "Expected to remain in displayed pixels, subject to the site's crop and image pipeline. "
            "Not upload/download tested in this review.",
        ),
    },
    "Google Images": {
        "exif": cell(
            UNKNOWN, INFERENCE,
            "Google's supported image-metadata documentation enumerates IPTC rights/source fields, "
            "not general camera/GPS EXIF. Google Images crawls a source URL rather than returning an "
            "upload derivative.",
            GOOGLE_METADATA,
        ),
        "iptc_xmp": cell(
            READS, DOCUMENTED,
            "Google documents extracting specific IPTC creator, credit, copyright, licence, "
            "licensor, and digital-source fields from images it crawls. It does not promise to "
            "display every field or every eligible result.",
            GOOGLE_METADATA, GOOGLE_DETAILS,
        ),
        "c2pa": cell(
            PARTIAL, DOCUMENTED,
            "Google documents using C2PA as one source for how an image was made or edited in image "
            "details. Availability and display are conditional; this is not upload survival.",
            GOOGLE_DETAILS,
        ),
        "visible": cell(
            KEEP, INFERENCE,
            "Google indexes the source image's pixels, but thumbnails and previews can crop or "
            "rescale a small edge stamp. This is not an upload/download path.",
        ),
    },
}

LAYERS = [
    ("visible", "Visible stamp", "Pixels; may be cropped, resized, softened, or screenshotted."),
    ("c2pa", "C2PA Content Credentials", "Signed provenance manifest and platform UI support."),
    ("iptc_xmp", "IPTC / XMP", "Creator, credit, copyright, rights, and source fields."),
    ("exif", "EXIF", "Camera, lens, date, orientation, and GPS."),
]

SYMBOL = {KEEP: "keeps", STRIP: "STRIPS", PARTIAL: "partial", READS: "reads", UNKNOWN: "unknown"}
EVIDENCE_SYMBOL = {DOCUMENTED: "D", CORROBORATED: "C", INFERENCE: "?"}


def summary_for(platform):
    return PLATFORMS.get(platform)


def display_cell(record):
    return f"{EVIDENCE_SYMBOL[record['evidence']]} {SYMBOL[record['verdict']]}"


def as_dict(include_notes=False, include_check=False):
    report = {
        "researched": RESEARCHED,
        "legend": {"D": DOCUMENTED, "C": CORROBORATED, "?": "unverified inference"},
        "layers": [{"key": key, "label": label, "description": description}
                   for key, label, description in LAYERS],
        "platforms": {},
        "advice": one_line_advice(),
    }
    for name, layers in PLATFORMS.items():
        report["platforms"][name] = {}
        for key, _, _ in LAYERS:
            record = layers[key]
            item = {
                "verdict": record["verdict"],
                "evidence": record["evidence"],
                "live_tested": record["evidence"] == CORROBORATED,
            }
            if include_notes:
                item.update({"note": record["note"], "sources": record["sources"]})
            report["platforms"][name][key] = item
    if include_check:
        report["how_to_verify"] = how_to_verify()
    return report


def one_line_advice():
    return (
        "Pixels are the only broadly portable layer. LinkedIn documents C2PA display, but rollout "
        "and self-signed credential handling are unverified. Test your exact upload path."
    )


def how_to_verify():
    return """Verify a row with the exact account, app, file type, and upload path you use:

  1. credit photo.jpg --creator "Your Name" --copyright "© Your Name" \\
       --stamp "Your Name" --sign --digital-source camera
  2. Save `snitch --json photo-credited.jpg` as the before report.
  3. Upload it to the platform and record whether a provenance/AI label appears.
  4. Download the highest-quality public derivative; do not use your retained local original.
  5. Save `snitch --json downloaded.jpg` and compare every layer with the before report.

Platform behaviour varies by feed/story/ad/message route, client, account, format, and date. A label
shown by the platform proves ingestion, not that the downloadable derivative kept the metadata."""
