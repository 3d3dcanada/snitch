# Claude Work Order: Independent SNITCH Audit and Implementation Readiness

**Prepared:** 2026-08-29  
**Mode:** Audit and handoff only. Do not implement, commit, push, deploy, alter runtime configuration, or publish anything in this pass. Save your report as a new dated document in `/home/wess/snitch/docs/`; leave application code unchanged.  
**Authority:** The user requested a second independent audit after Gemini's review. This does not authorize Gemini's two proposed implementation packages.

## Copy/paste invocation

~~~text
Read /home/wess/snitch/docs/HANDOFF-CLAUDE-SNITCH-AUDIT-2026-08-29.md in full, then independently audit the current SNITCH Python package and 3d3d-site SNITCH web app against /home/wess/snitch/docs/EXTERNAL-INPUT-VERIFICATION-AND-HANDOFF.md. Audit only: do not edit implementation files, commit, push, deploy, or claim unobserved browser/live behavior. Re-run every feasible gate, actively try to disprove privacy, integrity, C2PA, packaging, MCP, and routing claims, and save a dated evidence-backed report in /home/wess/snitch/docs/. Start with the required state and project-rule checks in the handoff. Report exact commands, outputs, findings by severity, unverified gaps, and a single safe next work order.
~~~

## Purpose and source material

Audit whether Gemini's proposal is technically sound and whether either requested package already exists. Treat all of these as inputs rather than proof:

1. `/home/wess/snitch/docs/EXTERNAL-INPUT-VERIFICATION-AND-HANDOFF.md` is Gemini's untracked proposal. Do not overwrite it. It asks for:
   - Python stdio MCP tools: `snitch_inspect`, `snitch_strip_metadata`, `snitch_add_credit`, `snitch_verify_c2pa`, and `snitch_clean_text`.
   - `snitch-mcp` and `snitch mcp`, MCP tests, clean pytest/mypy/Ruff, and README setup for Claude, Cursor, and Antigravity.
   - a web Studio layout, developer hub, visible SHA-256/canvas integrity proof, direct intent routing, and strict C2PA remote-manifest opt-out.
2. This work order records a first audit. Reproduce its findings, do not defer to them.
3. Historical evidence is context, not current proof:
   - `/home/wess/snitch/docs/work-orders/audit-2026-08-25/evidence/`
   - `/home/wess/3d3d-site/docs/HANDOFF-2026-08-27-SNITCH-COMPLETE.md`
   - `/home/wess/3d3d-site/docs/HANDOFF-2026-08-25-SNITCH-TOOLS.md`

## Required state and project-rule checks

Before code inspection, read in full:

- `/home/wess/3d3d-site/AGENTS.md`
- `/home/wess/3d3d-site/CLAUDE.md`
- this work order
- Gemini's proposal

Then record:

~~~bash
git -C /home/wess/snitch status --short --branch
git -C /home/wess/3d3d-site status --short --branch
git -C /home/wess/snitch remote -v
git -C /home/wess/3d3d-site remote -v
~~~

State observed on 2026-08-29 before this work order was written:

- `/home/wess/snitch`: `main...origin/main`, containing Gemini's untracked proposal only.
- `/home/wess/3d3d-site`: clean `codex/round-4` checkout.
- Both remotes are `https://github.com/3d3dcanada/<repo>.git`.

The site rules are binding: no unapproved redesign, invented claim, dark generic dashboard, em dash, or hidden data transfer. UI claims require visible verification in Ken's Brave through `claude-in-chrome`, never Playwright. Work the current site board in order. The current Panel was deliberately changed into a one-panel application after Ken rejected a stacked page of separate tools. A Studio Workstation must be reconciled with that decision and preserve the one-file workflow; this audit is not permission to decide the design.

## What is actually present

### Python package: `/home/wess/snitch`

- `snitch/mcp.py` is absent.
- `tests/test_mcp.py` is absent.
- `pyproject.toml` has only `snitch`, `no-comment`, and `credit` console scripts. It has neither `snitch-mcp` nor an MCP dependency.
- `snitch/cli.py` intentionally describes three direct commands and has no `mcp` subcommand dispatch.
- `core.strip()` supports only JPEG and PNG. It does container surgery and `pixels_identical()` calculates decoded-pixel SHA-256 with Pillow. Do not claim equivalent Python support for WebP, HEIC, or AVIF.
- `core.inspect()` already returns structured camera, GPS, credit, C2PA status, AI declaration, and errors. `core.read_c2pa_report()` distinguishes `present`, `absent`, `unavailable`, and `error`; keep that separation.
- `core.write_credit()` uses ExifTool and mutates the given file. An MCP adapter must never call it in place without safe, explicit output handling.
- There are 53 explicit `def test_` functions before pytest parameter expansion. That is not current passing-test evidence.

### Web app: `/home/wess/3d3d-site`

- `app/snitch/Panel.tsx` is a photo/text chooser containing existing Inspector, Sanitizer, Credit/stamp, Document analyzer, and Text scanner steps. It is not the proposed five-workspace Studio and has no developer hub.
- No SNITCH source uses `crypto.subtle.digest("SHA-256", ...)`. Scrub displays “pixels untouched, nothing re-compressed” but does not calculate or show a per-output canvas/pixel SHA-256 at runtime.
- `app/snitch/Inspector.tsx` passes both `fetchRemoteManifests: false` and `settings.verify.remoteManifestFetch: false` to C2PA. The installed type declarations accept both fields. `public/c2pa/c2pa.worker.min.js` and `public/c2pa/toolkit_bg.wasm` exist. This is source/build proof only; use a browser waterfall before calling remote-manifest privacy live-proven.
- The installed package is `c2pa@0.30.17`; its lockfile marks it deprecated and recommends `@contentauth/c2pa-web`. The official Content Authenticity project identifies the old c2pa-js repository as legacy. Do not represent the installed SDK as a sound long-term dependency just because its current opt-out compiles.
- `snitch-intent.tsx` embeds standalone `Inspector`, `Scrub`, `Credit`, or `Documents`, not the shared Panel. The tools work in page, but the named intent routes do not enter the requested active Panel state.
- `/pdf-metadata-remover` is currently a 301 to `/does-a-pdf-show-who-made-it`, whose page embeds Documents.
- No web developer/release/MCP-config hub exists. Existing source mentions `pip install snitch-tools` only in documentation.

## Corrections that must survive all future work

### File-local processing is not zero server transfer

The valid product promise is that file bytes, names, hashes, EXIF, location, and text stay in the browser. But when consent permits, `app/lib/analytics.ts` sends `snitch_tool` events to `https://3d3d-analytics.3d3dcanada.workers.dev/event`. Optional Feedback sends a tool label, yes/no answer, and up to 240 characters of visitor text through the same analytics path. DNT/GPC and consent disable it, and the event contract deliberately excludes all file data, but traffic still occurs.

Copy must say “your file stays on this device” or “no file is uploaded.” Never say “zero server data transfer,” “no network requests,” or “nothing is sent” while analytics/feedback remains.

### A visual hash must say precisely what it proves

The current web UI does not perform a per-run proof. Historical fixtures and container-surgery logic are useful but are not a user-visible SHA-256 result. A canvas digest proves only decoded canvas raster output after browser decode; it can differ because of color management or orientation. It is not the raw file hash, proof all metadata is removed, C2PA validity, or a proof of identity.

For each format, a future UI must show an honest state: for example, “verified decoded pixels identical,” “container guarantee, not decoded in this browser,” or “not verified.” Add fixtures with a deliberately changed-pixel failure. Never turn a green hash badge into a generic trust signal.

### MCP compatibility needs an owner decision first

The current official MCP Python SDK requires Python 3.10+, while SNITCH declares `requires-python = ">=3.9"`. Do not silently raise the package floor and do not pin a stale SDK to retain 3.9.

The owner must choose one of:

1. raise SNITCH itself to Python 3.10+ and make MCP required;
2. retain Python 3.9 and defer the official MCP server;
3. ship a separately installed, clearly Python-3.10+ MCP extra, if packaging/support policy approves it.

Use the official `mcp` Python SDK and local stdio only. Its stdout is protocol wire: no `print()`, banner, ANSI output, or subprocess noise may corrupt it. Log only to stderr. Test with an actual MCP client/server session.

### C2PA integrity, trust, and timestamps are different states

Return raw structured validation plus independent summaries for:

- manifest present/absent/tool unavailable/parse error;
- asset-binding integrity, valid/invalid/unknown;
- signing-certificate trust and identity;
- timestamp validity, only when the validator actually reports it.

Do not treat `signingCredential.untrusted` as altered, do not treat a self-signed credential as verified identity, and do not read missing C2PA as evidence that an image is not AI-generated. Metadata stripping neither detects nor removes in-pixel watermarks.

## First audit: commands and exact outcomes

Repeat these with an isolated environment. The first audit observed:

| Command | Observed outcome |
| --- | --- |
| `npm run typecheck` in `/home/wess/3d3d-site` | Exit 0. Wrangler emitted `envFile` deprecation and Node emitted `punycode` deprecation warnings. |
| `npm run build` in `/home/wess/3d3d-site` | Exit 0. Checkout stayed clean. Vite warned about chunks above 500 kB. The C2PA client chunk was 338.90 kB minified and 117.42 kB gzip. |
| `mypy snitch` in `/home/wess/snitch` | Exit 1: `snitch/core.py:277: error: Skipping analyzing "PIL": module is installed, but missing library stubs or py.typed marker [import-untyped]`; one error in one file. |
| `python -m pytest -v` | Did not run: `python: command not found`. |
| `ruff check snitch` | Did not run: `ruff: command not found`. |
| `python3 -m pytest -v` | Did not run: `/usr/bin/python3: No module named pytest`. |
| `python3 -m ruff check snitch` | Did not run: `/usr/bin/python3: No module named ruff`. |

The default Python was `/usr/bin/python3` 3.12.3, with Pillow 10.2.0 and without pytest/Ruff. Mypy 1.19.1 was installed under the user site. Thus passing tests and a clean Ruff gate are unobserved, while clean mypy is actively disproved.

Use a disposable venv under `/tmp`, install `-e '.[dev]'`, then run pytest, Ruff, mypy, build, and installed-wheel CLI smoke tests. Dependency installation may need network approval. Report actual `c2patool` and ExifTool availability. Do not claim unobserved live platform uploads, browser waterfalls, or C2PA validation as passed.

## Required adversarial audit procedure

1. Re-check repository state and read all instructions/proposal in full.
2. Map every Gemini request to **present**, **absent**, **contradicted**, or **unverified**, with source-and-line evidence. Historical docs and commit messages never count as implementation.
3. Build the temporary Python environment and record Python version, resolved dependency versions, ExifTool, and c2patool. If you suggest `types-Pillow`, distinguish an actual packaging fix from merely suppressing mypy.
4. Exercise real safe fixtures: JPEG/PNG carrying EXIF/GPS, IPTC/XMP, C2PA if tools permit, Unicode metadata, option-like filename, malformed file, output collision, and symlink. Verify source preservation and no partial artifacts after failure.
5. Inspect C2PA source, its installed API types, worker/WASM assets, object URL cleanup, and SDK disposal. Prove the current route/component behavior rather than inferring it from route names.
6. In Ken's Brave only, use real files through Inspector, Scrub, Credit, Documents, and Text Scanner at 390px and desktop. Capture Network. Separate static assets, consent-region lookup, analytics/feedback, C2PA resources, and any remote-manifest request. Do not use a GPC-blocked session as proof of zero transfer.
7. Test `/remove-exif-online`, `/add-copyright-to-photo`, and `/pdf-metadata-remover` directly. Record redirect/component, active tool state, file behavior, and whether Panel is used.
8. Check GitHub releases before proposing release download buttons. Do not create a fake download control if no release exists. Verify every host-specific MCP configuration from current official host documentation; never invent an Antigravity schema.
9. Classify P0 correctness/privacy, P1 packaging/support, P2 UX/design, and evidence gaps. End with exactly one focused next work order.

## Later implementation definition of done

Do not perform this section without fresh implementation authorization and an explicit Python compatibility decision.

### Python and MCP

- Build a thin adapter over tested core behavior, not parallel business logic. Refactor CLI-only safety code into a public service layer if needed.
- Require ordinary source files; explicitly refuse destructive symlink operations; resolve output paths safely; prevent source overwrite; retain atomic sibling writes, collision checks, and `--force` behavior.
- Mutation tools must return output path, format, byte delta, removed classes, and honest proof state. Never return file bytes or mutate an input silently. Make stamping explicit because it re-encodes and invalidates C2PA.
- `snitch_clean_text` needs documented Python behavior and a test matrix that protects emoji ZWJ sequences and legitimate Arabic, Persian, Kurdish, and Indic ZWNJ/ZWJ use while removing true tracking controls. Do not call non-ASCII suspicious.
- `snitch mcp` and `snitch-mcp` must run the same local stdio server with zero stdout before/during protocol startup. Test discovery, schemas, each tool success path, malformed request, c2patool unavailable, and error handling through a real MCP client. Stop every server process you start.
- README setup must be current and host-verified. State Python, MCP, ExifTool, c2patool, and supported-format limits truthfully.

### Web Studio

- Preserve local file processing and the no-file-data analytics contract. If analytics remains, say “file stays on this device,” not “no data is sent.” Removing analytics is a separate measurement decision.
- Keep remote manifests disabled. Add regression/browser evidence that a file-derived remote URL is never fetched. Migrate the deprecated C2PA SDK separately with fixture tests, not inside a cosmetic Studio rewrite.
- A workspace switcher needs semantic tabs, keyboard navigation, selected state, focus visibility, and a single preserved selected file. It must work at 390px and desktop and must not convert the approved warm app into a generic dark dashboard.
- Developer hub URLs must be verified, canonical, and accessible. Install commands and MCP snippets are documentation, not proof of installation. No secret, network server, or invented host schema.
- Integrity UI must label algorithm, input, limitation, and result. It must never conflate file hash, canvas digest, C2PA validity, and identity.
- Run typecheck/build, then visible Brave interaction, console, and Network proof. Do not commit, deploy, or publish unless newly authorized.

## Required Claude audit deliverable

Save `/home/wess/snitch/docs/AUDIT-CLAUDE-SNITCH-YYYY-MM-DD.md` containing:

1. executive verdict: ready, conditionally ready, or not ready;
2. exact states and instruction files read;
3. request-to-evidence matrix;
4. commands plus meaningful exact outputs, including failures;
5. strict separation of source/build/fixture/browser/live proof;
6. every privacy, C2PA, integrity, compatibility, routing, and design finding;
7. one safe next work order and owner decisions required first.

Never call a proposal complete, telemetry zero transfer, a self-signed credential trusted, metadata stripping watermark removal, or an unrun quality gate clean.

