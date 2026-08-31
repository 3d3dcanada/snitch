# SNITCH · External Input Verification & Cross-Agent Handoff

> **Prepared for**: Randall Marshall, Codex, and Claude  
> **Date**: 2026-08-29  
> **Topic**: Technical verification of external AI/Reddit guidance against SNITCH codebase + cross-agent handoff for Phase 1 & Phase 2 execution.

---

## Part 1: Verification of External AI Chatbot Input

### 1. Verbatim Input Received

```text
Ah yes, the digital witness protection program for generated images. Nothing saves face quite like scrubbing out a 600-word prompt containing (ugly, mutated hands, extra digits, nightmare teeth:1.4) before showing off your "hand-crafted masterpiece" to the timeline.

Jokes aside, building custom utilities for this is awesome. As someone who lives in a server rack and respects a good data diet, here are a few technical tidbits and edge cases worth keeping on your radar as you refine it:

PNG Chunks vs. C2PA Manifests: Standard Stable Diffusion / ComfyUI setups inject workflow metadata into PNG text chunks (tEXt, zTXt, iTXt). Meanwhile, tools like DALL-E 3 and Adobe Firefly bake in C2PA Content Credentials via JUMBF and XMP blocks. If your tool scrubs both chunk types cleanly, you're ahead of 90% of basic image compressors. You can benchmark your outputs against the Content Authenticity Verify tool to confirm C2PA manifests are actually gone.

Invisible Text Fingerprints: For text metadata, keep an eye out for zero-width unicode characters (\u200B, \u200C, etc.) and non-standard whitespace. Some AI platforms sneak invisible tracking characters into copied text outputs, so a quick regex sweep to sanitize non-printable unicode is a killer feature.

Metadata vs. Pixel-Level Watermarks: It’s worth putting a quick disclaimer in your UI reminding users that metadata stripping removes header info, but won't alter perceptual pixel-domain watermarks (like DeepMind's SynthID or Digimarc).

The Custom Injection Feature: Being able to replace machine headers with your own clean IPTC/EXIF copyright tags is genuinely handy for creators who want clean attribution without workflow bloat. If you ever need reference standards for structured tags, the ExifTool Tag Directory is basically the holy scripture of metadata schema.

Kudos for taking the DIY route and sharing it with the class! Keep tinkering with it—indie utility tools are the backbone of messy creative workflows.
```

---

### 2. Point-by-Point Technical Verification

#### Claim A: PNG Text Chunks (`tEXt`, `zTXt`, `iTXt`) vs C2PA Manifests (`caBX`, JUMBF, XMP)
* **Chatbot Assessment**: Stable Diffusion and ComfyUI bake parameters into PNG text chunks (`tEXt`, `zTXt`, `iTXt`), while DALL-E 3 and Firefly embed C2PA manifests via JUMBF (`caBX` in PNG, APP11 in JPEG). Losslessly stripping both without re-encoding puts a tool ahead of 90% of compressors.
* **Our Implementation & Code Proof**:
  * In Python core [`snitch/core.py:304-353`](file:///home/wess/snitch/snitch/core.py#L304-L353) (`strip_png`): We filter PNG chunks by whitelist. Critical chunks (`IHDR`, `PLTE`, `IDAT`, `IEND`, `tRNS`) and display chunks (`cHRM`, `gAMA`, `iCCP`, `sRGB`, `sBIT`, `acTL`, `fcTL`, `fdAT`) are retained. All non-display ancillary chunks (`tEXt`, `zTXt`, `iTXt`, `caBX`, `eXIf`) are dropped. Decoded pixels are verified byte-identical via SHA-256 canvas digest.
  * In JPEG [`snitch/core.py:207-265`](file:///home/wess/snitch/snitch/core.py#L207-L265) (`strip_jpeg`): All application markers (`APP1` EXIF/XMP, `APP11` JUMBF/C2PA, `APP13` IPTC, `COM` comments) are stripped, while preserving appearance-critical `APP0` (JFIF), `APP2` (ICC profile), and `APP14` (Adobe CMYK transform).
  * In Web Client [`Scrub.tsx:76-187`](file:///home/wess/3d3d-site/app/snitch/Scrub.tsx#L76-L187): Identical chunk-filtering logic runs client-side in TypeScript across JPEG, PNG, WebP (RIFF), and HEIC/AVIF (ISOBMFF).
* **Verdict**: **100% Corroborated & Already Handled.**

#### Claim B: Invisible Text Fingerprints (Zero-Width Unicode & Whitespace)
* **Chatbot Assessment**: Sanitize zero-width characters (`\u200B`, `\u200C`, etc.) and non-standard whitespace to prevent tracking in copied text.
* **Our Implementation & Code Proof**:
  * In [`lib/text.ts:45-84`](file:///home/wess/3d3d-site/app/snitch/lib/text.ts#L45-L84) and [`TextMark.tsx:47-110`](file:///home/wess/3d3d-site/app/snitch/TextMark.tsx#L47-L110): We detect framed identifiers, suspicious standalone zero-width characters (`\u200B` ZWSP, `\uFEFF` BOM, `\u2060` WJ, `\u2063` Invisible Separator, `\u2061` Function Application), odd whitespace (`\u00A0`, `\u2007`, `\u2009`, `\u202F`, `\u3000`, `\u200A`), bidi overrides, and mixed-script confusables.
  * **Critical Edge Case We Solved That Naive Regex Misses**: Indiscriminate stripping of `\u200C` (ZWNJ) and `\u200D` (ZWJ) destroys emojis (e.g. `👩‍💻`) and legitimate orthography in Arabic, Persian, Kurdish, and Indic languages. SNITCH protects legitimate script usage while stripping adversarial tracking markers.
* **Verdict**: **Corroborated & Exceeded.**

#### Claim C: Metadata Stripping vs In-Pixel Watermarks (SynthID / Digimarc)
* **Chatbot Assessment**: Make sure to disclaim that header stripping removes metadata but cannot remove in-pixel perceptual watermarks like Google SynthID or Digimarc.
* **Our Implementation & Code Proof**:
  * In [`snitch/README.md:165-174`](file:///home/wess/snitch/README.md#L165-L174), [`Scrub.tsx:446-449`](file:///home/wess/3d3d-site/app/snitch/Scrub.tsx#L446-L449), [`routes/snitch.tsx:56-58`](file:///home/wess/3d3d-site/app/routes/snitch.tsx#L56-L58), and [`CLAUDE.md:33-40`](file:///home/wess/3D3D-BRAIN/CLAUDE.md#L33-L40): This is an enforced house law. We explicitly state in CLI help and web UI that perceptual watermarks are embedded in image data and cannot be removed by metadata stripping.
* **Verdict**: **100% Corroborated & Already Strictly Enforced.**

#### Claim D: Structured Injection & ExifTool Tag Schemas
* **Chatbot Assessment**: Provide clean IPTC/EXIF injection; use ExifTool Tag Directory as the schema reference.
* **Our Implementation & Code Proof**:
  * In [`snitch/core.py:405-453`](file:///home/wess/snitch/snitch/core.py#L405-L453) (`write_credit`): We use ExifTool tag standards for IPTC Core, XMP Dublin Core, Photoshop, PLUS, and EXIF fields.
* **Verdict**: **100% Corroborated & Fully Functional in CLI.**

---

## Part 2: Cross-Agent Handoff (For Codex & Claude)

### Current Status
* **Core Codebase**: Standalone Python package in [`/home/wess/snitch`](file:///home/wess/snitch).
* **Website Component**: React Router / Vite app in [`/home/wess/3d3d-site/app/snitch/`](file:///home/wess/3d3d-site/app/snitch/).
* **Active Branches**:
  * `snitch`: clean `master` on 3d3dcanada remote.
  * `3d3d-site`: active website development.

### Work Order 1: Standalone Python Engine & MCP Server (`/home/wess/snitch`)
1. **Implement Native MCP Server** ([`snitch/mcp.py`](file:///home/wess/snitch/snitch/mcp.py)):
   * Expose stdio-based MCP tools:
     - `snitch_inspect(file_path)`: Return structured JSON of camera, GPS, credit, and C2PA integrity.
     - `snitch_strip_metadata(file_path, out_path=None)`: Lossless byte surgery strip.
     - `snitch_add_credit(file_path, creator, credit, copyright, ...)`: Inject IPTC/XMP tags + optional stamp.
     - `snitch_verify_c2pa(file_path)`: Check C2PA manifest validity and certificate trust status.
     - `snitch_clean_text(text)`: Sanitize zero-width tracking characters while preserving emojis.
   * Add entry point `snitch-mcp` and CLI subcommand `snitch mcp` in [`pyproject.toml`](file:///home/wess/snitch/pyproject.toml).
2. **Quality & Packaging**:
   * Add test suite in `tests/test_mcp.py`.
   * Verify all tests pass: `pytest -v`.
   * Verify type safety: `mypy snitch`.
   * Verify linting: `ruff check snitch`.
3. **Documentation**:
   * Update [`README.md`](file:///home/wess/snitch/README.md) with PyPI install instructions (`pip install snitch-tools`), CLI usage, MCP setup config for Claude Desktop / Cursor / Antigravity, and platform survival research.

### Work Order 2: Web Application Studio Upgrade (`/home/wess/3d3d-site`)
1. **Workstation Studio Layout** ([`app/snitch/Panel.tsx`](file:///home/wess/3d3d-site/app/snitch/Panel.tsx)):
   * Upgrade layout to feel like a high-utility desktop-grade web application.
   * Add dedicated workspace switcher: **Inspector**, **Sanitizer**, **Watermark Studio**, **Document Analyzer**, **Text Scanner**.
   * Display real-time visual hash / canvas byte-integrity proof on sanitized output.
2. **Developer & Open Source Hub**:
   * Add top-level GitHub repository badge and release download links.
   * Add one-click copy command for `pip install snitch-tools`.
   * Add MCP Configuration snippet modal / copyable JSON for AI assistant users.
3. **Audit Hardening**:
   * Ensure `fetchRemoteManifests: false` is strictly set on C2PA JS SDK.
   * Verify intent routes (`/remove-exif-online`, `/add-copyright-to-photo`, `/pdf-metadata-remover`) route directly to the active tool panel.
