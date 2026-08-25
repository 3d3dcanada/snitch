# 2026 landscape and missing capability

Research was performed 2026-08-25 from current primary specifications, vendor documentation, and
maintainer documentation. It is not a substitute for platform round trips. No files or account
data were uploaded during this work.

## Comparable tools

| Tool | What it does that snitch does not | Honest opening for snitch |
|---|---|---|
| [ExifTool](https://exiftool.org/exiftool_pod.html) | Reads/writes hundreds of tag groups and far more image, RAW, video, audio, and document formats; supports batch argument files and advanced copying. | It is a low-level metadata engine. Snitch can add opinionated privacy/rights policies, plain trust output, safe defaults, and retained pixel/display proofs. Snitch already depends on it and should not pretend to replace it. |
| [Exiv2](https://exiv2.org/manpage.html) | Native library/CLI, XMP and EXV sidecars, metadata copy workflows, broad image/RAW handling. | Snitch can remain simpler and provenance-focused, but currently lacks sidecars and serious RAW workflow support. |
| [mat2](https://manpages.debian.org/unstable/mat2/mat2.1.en.html) | Cleans images, audio, office files, PDFs, and archives; offers `--show`, `--check-dependencies`, lightweight mode, and archive-member policy. | mat2 explicitly admits it cannot detect every field and that cleaning can cost quality. Snitch can beat it narrowly for JPEG/PNG by proving decoded equality and preserving display-critical data, not by claiming broader removal. |
| [ExifCleaner](https://github.com/szTheory/exifcleaner) | Desktop drag/drop, recursion, before/after diff, ExifTool stay-open batch speed, 90+ formats, 25 languages, timestamp/orientation policies, and installed-app E2E tests on all three OS families. | Its current limitations are unusually candid. Snitch's distinction is credit writing, visible stamping, C2PA inspection, and a sourced platform index; it is behind on batch UX, format breadth, and cross-platform evidence. |
| [c2patool / CAI SDKs](https://opensource.contentauthenticity.org/docs/getting-started/inspect/) | Current manifest inspection/signing, ingredients/actions, broader image/audio/video/PDF types, SDK integrations, and trust/conformance machinery. | Snitch provides a simpler creator-facing wrapper, but its current signing recipe is legacy and should use the SDK rather than flattening modern C2PA into one self-signed command. |
| [Adobe Content Authenticity Beta](https://helpx.adobe.com/creative-cloud/apps/adobe-content-authenticity/beta-overview.html) | Verified names/social accounts, AI-use preferences, cloud manifests, invisible watermark plus fingerprint recovery, and up to 50 JPG/PNG files per batch. | It requires account/cloud upload and currently applies only to JPG/PNG up to 20 MB. Snitch is local, inspectable, and broader for metadata-only credit, but has no durable or verified identity. |

The overclaiming opportunity is real but should be stated narrowly. For example, the browser tool
[i2IMG](https://www.i2img.com/remove-image-metadata) promises EXIF/IPTC/XMP “and more,” “lossless”
output, and unchanged visible pixels, but publishes no format-specific exceptions, ICC/orientation
policy, or retained proof.
[ImageMagick's `-strip`](https://imagemagick.org/script/command-line-options.php#strip) can remove
profiles needed for colour rendering.
Snitch can credibly say exactly which JPEG/PNG structures it removes, which display structures it
keeps, and show a decoded hash. It should not say “all metadata” or generalize that proof to other
formats.

## Standards that moved

### C2PA and Content Credentials

The current [C2PA technical specification 2.4](https://spec.c2pa.org/specifications/specifications/2.4/specs/C2PA_Specification.html)
was published in April 2026. Since 2.3 in December 2025 it has added live/dynamically packaged
video, unstructured and structured text embedding, OGG/large AVI/Original Preservation Image
support, external references, fine-grained watermark actions, crJSON reporting, repository
receipts, AI disclosure, sustainability data, improved TIFF handling, and formal soft-binding
publication/recovery routes. Version 2.4 also says mandatory actions belong in created assertions
and recommends `specVersion` in `claim_generator_info`.

Snitch currently signs a claim-v2 asset binding through c2patool, but its recipe contributes
`stds.schema-org.CreativeWork` and the action as gathered assertions. The Schema.org assertion has
been deprecated since C2PA 2.0. It does not declare a normative C2PA version, model ingredients,
create a modern metadata assertion, expose crJSON, use repository receipts/soft bindings, or
support the new AI disclosure. It must not add `specVersion: 2.4.0` until those incompatibilities
are fixed.

The official [C2PA Conformance Program](https://c2pa.org/conformance/) and Trust List launched in
mid-2025; the Interim Trust List was frozen on 2026-01-01. A conforming generator has security and
certificate requirements and signs with an eligible X.509 certificate chained to the official
list. The [CAWG Identity Assertion 1.3](https://cawg.io/identity/1.3/) was ratified on 2026-08-17
and is the current route for a human or organization to bind identity to referenced assertions.
Snitch has neither conformance nor CAWG identity. CAI also says filesystem private keys are fine
for development but production signing should use a KMS or HSM.

### IPTC Photo Metadata

The current [IPTC Photo Metadata Standard 2025.1](https://www.iptc.org/std/photometadata/specification/IPTC-PhotoMetadata-2025.1.html)
added AI Prompt Information, AI Prompt Writer Name, AI System Used, and AI System Version Used.
IPTC notes that ExifTool supports these only from 13.40; the audited Ubuntu package is 12.76.
Snitch neither exposes the fields nor diagnoses that version gap.

Older standard fields also missing from the CLI matter in real delivery: Alt Text Accessibility,
Extended Description Accessibility, location created/shown, people and products shown, event,
job/instructions, artwork/object detail, model/property release IDs and status, PLUS rights,
Data Mining, Other Constraints, and encoded rights expressions.

IPTC's current [Digital Source Type vocabulary](https://cv.iptc.org/newscodes/digitalsourcetype/)
distinguishes camera, computational capture, film/print digitization, human edits,
algorithmically-enhanced media, human digital creation, generative AI, composites, screen capture,
and virtual recordings. Snitch offers only eight aliases and writes them only into the C2PA action;
it does not write `Iptc4xmpExt:DigitalSourceType` into ordinary XMP, where search/social systems can
read it even without a credential.

### Platform handling

The table is already missing important platforms and newer display behaviours:

- [Google's May 2026 update](https://blog.google/innovation-and-ai/products/identifying-ai-generated-media-online/)
  expands SynthID verification to Search and C2PA verification to Gemini, with Search/Chrome
  rollout announced, and says Instagram will begin labelling camera-captured Pixel media.
- [YouTube](https://support.google.com/youtube/answer/15446725) requires C2PA 2.1 or later and an
  unedited provenance chain for its “Captured with a camera” disclosure. Snitch's self-signed
  edited-image credential is not that workflow.
- [TikTok](https://newsroom.tiktok.com/partnering-with-industry-to-advance-ai-transparency-and-literacy?lang=en-GB)
  reads Content Credentials for automatic AI labels; its November 2025 update also describes
  adding invisible watermarks to C2PA-labelled uploads because metadata can disappear on reupload.
- [Pinterest](https://newsroom.pinterest.com/news/introducing-gen-ai-labels/) introduced worldwide
  AI-modified labels in 2025 using metadata plus classifiers.

TikTok, YouTube, Pinterest, Behance, Flickr, Adobe Stock, ArtStation, Etsy, Mastodon, and Bluesky
are candidates for the table, but none should be added with a survival verdict until the exact
route and derivative are tested or the platform documents that narrower behaviour.

## Missing creator formats and workflows

Photographers expect non-destructive XMP sidecars for proprietary RAW files, metadata presets at
ingest, copy/merge templates, keyword hierarchies, and write-back to DNG/PSD/TIFF/JPEG. [Lightroom
Classic 15](https://helpx.adobe.com/lightroom-classic/desktop/organize-photos-in-lightroom-classic/create-xmp-acr-files.html)
added separate ACR sidecars for heavy edits in October 2025 while retaining XMP for ordinary
metadata. [Photo Mechanic](https://docs.camerabits.com/support/solutions/articles/48000207623-using-the-metadata-iptc-template)
supports reusable templates, variables, and code replacements. Snitch has no sidecar, ingest,
preset, catalog, or RAW policy. At minimum it is missing RAW/DNG, PSD/PSB, AVIF, JPEG XL, GIF/MPO,
and XMP sidecars; editing RAW originals should be refused by default.

Illustrators expect PSD/PSB, Adobe Illustrator/PDF, SVG metadata, AVIF/JPEG XL/WebP exports,
multi-language XMP, accessibility descriptions, source/ingredient attribution, and rights that
survive export. [SVG 2](https://www.w3.org/TR/SVG/struct.html#MetadataElement) provides a standard
`<metadata>` container, but snitch does not inspect it.

3D designers need provenance on the model and package, not only its preview JPEG. The
[3MF Core Specification](https://3mf.io/wp-content/uploads/sites/106/2025/02/3MF_Core_Specification_v1.3.0.pdf)
defines Title, Designer, Description, Copyright, LicenseTerms, dates, and source Application.
[glTF](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html) has asset copyright/generator and
the ratified
[KHR_xmp_json_ld](https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_xmp_json_ld/README.md)
extension for attribution/licensing at asset, scene, mesh, material, image, and animation scope.
[OpenUSD](https://openusd.org/release/api/class_usd_object.html) exposes `assetInfo` and custom
metadata. STL has no comparable native rich-metadata path, so a linked sidecar/package manifest is
the honest approach. Snitch supports none of these.

## Ranked additions

These are proposals only. The high-effort items were not built.

| Rank | Addition | Value | Effort | Reason |
|---:|---|---|---|---|
| 1 | Replace the signing recipe with current C2PA 2.4 created assertions, modern `c2pa.metadata`, ingredients/actions, trust-list evaluation, and optional CAWG 1.3 identity/remote signer. | Critical | High | The present `--sign` path is explicitly development-grade and blocks an honest claim of modern creator identity. |
| 2 | Build a retained platform round-trip harness: generated layer fixtures, upload-route manifest, downloaded derivatives, hashes/ExifTool/c2patool reports, date/account/app/format fields, and automatic table ingestion. | Very high | Medium | This is the shortest route from a careful research index to a publishable survival reference. Account actions still require explicit authorization. |
| 3 | Add `snitch doctor` for ExifTool/c2patool/OpenSSL versions, supported tags/formats, Pillow, key permissions, and clear capability flags. | High | Low | Daily failures currently appear only when work starts; IPTC 2025.1 needs ExifTool 13.40+. |
| 4 | Write the full current Digital Source Type into XMP and add IPTC 2025.1 AI fields with version-gated errors. | High | Medium | Platforms can read source metadata without C2PA, and the current eight C2PA-only aliases omit common capture/edit workflows. |
| 5 | Add policy presets and `--dry-run`: `privacy`, `rights-preserving`, and `all-private`, with a before/after plan and stable exit-code documentation. | High | Medium | “Remove metadata” is not one policy; creators need to keep ICC/orientation/rights while dropping GPS/device IDs. |
| 6 | Add XMP sidecar read/write and metadata presets, refusing proprietary RAW mutation by default. | High | Medium | This matches Lightroom/Photo Mechanic workflows without risking irreplaceable originals. |
| 7 | Expand credit fields to accessibility, caption/headline/instructions/job, location, people/products, releases, PLUS rights, data-mining, and other constraints. | High | Medium | These are ordinary professional delivery requirements, not edge metadata. |
| 8 | Extend inspection/write policies to DNG/RAW, PSD/PSB, AVIF, JPEG XL, GIF/MPO, SVG, PDF, video/audio, with per-format capability statements rather than one blanket promise. | Medium-high | High | Current coverage is too narrow for photographers and illustrators; format-specific safety is the expensive part. |
| 9 | Add a 3D provenance package: native 3MF fields, glTF `KHR_xmp_json_ld`, USD metadata, and a versioned sidecar for STL/OBJ plus linked previews. | High for this audience | High | A product/model creator needs attribution on the asset being distributed, not just screenshots. |
| 10 | Add recursive/NUL-delimited input, output-name templates, `--quiet`, per-file JSON mutation results, and final success/failure/skip counts. | Medium-high | Low-medium | These are the largest daily CLI annoyances after correctness. |
| 11 | Use ExifTool stay-open for large batches and stream or delegate the pixel proof to cap 100 MP memory further. | Medium | Medium | ExifCleaner demonstrates the batch-speed bar; 408 MiB is still high. |
| 12 | Add durable credentials through soft bindings, repository receipts, and optional watermark/fingerprint recovery. | Medium-high | Very high | It survives platform metadata loss, but introduces services, privacy policy, algorithm, and operating-cost decisions. |
| 13 | Add config/presets, shell completion, and a safe orphan-temp scanner/recovery command. | Medium | Low-medium | Repeated identity flags and stranded kill-time temp files are avoidable friction. |
