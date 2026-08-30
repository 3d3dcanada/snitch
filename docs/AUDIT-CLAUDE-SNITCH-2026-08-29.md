# Independent SNITCH Audit · Claude

**Date:** 2026-08-29
**Auditor:** Claude (Opus 5), second independent pass after Gemini and Codex
**Mode:** Audit only. No implementation file was edited, nothing was committed, pushed, deployed or published.
**Scope:** the Python package at `/home/wess/snitch` and the SNITCH web app in `/home/wess/3d3d-site`, against Gemini's proposal in `docs/EXTERNAL-INPUT-VERIFICATION-AND-HANDOFF.md` and the work order in `docs/HANDOFF-CLAUDE-SNITCH-AUDIT-2026-08-29.md`.

---

## 1 · Executive verdict

**Python package: CONDITIONALLY READY.** Every quality gate passes in a clean environment, the adversarial fixture matrix passes, and the C2PA handling is more honest than either prior document claimed. Two truth defects block publication, and one packaging defect makes the declared dependency floor unsound. Gemini's MCP package is entirely absent, and one of its five tools has no Python implementation to adapt.

**Web app: NOT AUDITABLE TO COMPLETION THIS PASS, AND NOT READY FOR GEMINI'S PACKAGE.** Another agent session held `/home/wess/3d3d-site` under live modification throughout this audit, confirmed by the user. I did not build, typecheck, or write in that repo. Source and live-site evidence is recorded below and is sound as of the timestamps given. Gemini's Studio proposal contradicts a standing design decision, and two of its three "audit hardening" items are already satisfied.

**The single most important correction in this report:** the first audit's only hard failure, `mypy` erroring on Pillow, is an environment artifact and not a code defect. It is disproved below. Acting on it with `types-Pillow` would suppress a symptom that does not exist.

---

## 2 · States and instruction files read

Read in full before code inspection:

- `/home/wess/3d3d-site/AGENTS.md` and `/home/wess/3d3d-site/CLAUDE.md`
- `/home/wess/snitch/docs/HANDOFF-CLAUDE-SNITCH-AUDIT-2026-08-29.md` (Codex work order)
- `/home/wess/snitch/docs/EXTERNAL-INPUT-VERIFICATION-AND-HANDOFF.md` (Gemini proposal)
- `/home/wess/snitch/README.md`, `pyproject.toml`, `snitch/cli.py`, `snitch/core.py`, `snitch/sign.py`, `snitch/survival.py`

### Repository state, observed 2026-08-29 23:39

```
$ git -C /home/wess/snitch status --short --branch
## main...origin/main
?? docs/EXTERNAL-INPUT-VERIFICATION-AND-HANDOFF.md
?? docs/HANDOFF-CLAUDE-SNITCH-AUDIT-2026-08-29.md

$ git -C /home/wess/snitch remote -v
origin	https://github.com/3d3dcanada/snitch.git (fetch/push)

$ git rev-parse HEAD origin/main
b773c5affe306f5afc8bc85ed444279ad6265e55
b773c5affe306f5afc8bc85ed444279ad6265e55
```

`/home/wess/snitch` matches the work order's recorded state exactly, and local `HEAD` equals `origin/main`, so what is published on GitHub is exactly what was audited.

### `/home/wess/3d3d-site` did NOT match the recorded state

The work order recorded "a clean `codex/round-4` checkout". It was not clean, and it changed while I looked at it:

```
23:39  ## codex/round-4      M app/content/CityPage.tsx        ?? app/content/RealWorkRail.tsx
23:41  ## codex/round-4      M app/content/RealWorkRail.tsx     M app/content/content-dates.generated.ts
```

Cause confirmed by process table and by the user: another agent session owns that repo right now.

```
$ ps -eo pid,etime,cmd | grep -E 'vite|wrangler|react-router'
1390530   04:01  npm exec wrangler dev --port 8833 --local     (cwd /home/wess/3d3d-site)
1384031   08:13  tsserver.js  (/home/wess/3d3d-site)
```

**Deliberate abstention.** I did not run `npm run typecheck` or `npm run build` in `/home/wess/3d3d-site`. `prebuild` runs `content-dates.mjs`, `image-variants.mjs` and `image-sizes.mjs`, which write generated files into a checkout another session is actively editing. The prior audit's "Exit 0" build result was obtained against a clean tree and is not reproducible against this one. This is recorded as an evidence gap in section 8, not as a pass or a fail.

### Environment actually used for the Python gates

A disposable venv, built fresh, as the work order directs:

```
$ python3 -m venv snitch-venv && ./snitch-venv/bin/pip install -e '/home/wess/snitch[dev]'
Successfully installed Pillow-12.3.0 build-1.6.0 mypy-1.20.2 pytest-9.1.1 ruff-0.16.5
                      twine-7.0.0 snitch-tools-0.1.0 ... (exit 0)
python 3.12.3 · Pillow 12.3.0
```

External tools, actual availability on this machine:

```
$ which exiftool  -> /usr/bin/exiftool           $ exiftool -ver      -> 12.76
$ which c2patool  -> /home/wess/.cargo/bin/c2patool   $ c2patool --version -> c2patool 0.27.15
```

Both are present. Every C2PA result below is a real validation, not a simulated one.

---

## 3 · Quality gates: the first audit's failures do not reproduce

| Command | First audit | This audit, clean venv | Verdict |
| --- | --- | --- | --- |
| `pytest -q` | did not run, no pytest | `65 passed in 15.96s` · exit 0 | **PASS** |
| `ruff check snitch` | did not run, no ruff | `All checks passed!` · exit 0 | **PASS** |
| `ruff check snitch tests` (as README instructs) | not attempted | `All checks passed!` · exit 0 | **PASS** |
| `mypy snitch` | **exit 1**, PIL import-untyped | `Success: no issues found in 6 source files` · exit 0 | **PASS, prior failure disproved** |
| `python -m build` | not attempted | `Successfully built snitch_tools-0.1.0.tar.gz and .whl` · exit 0 | **PASS** |
| `twine check dist/*` | not attempted | both artifacts `PASSED` | **PASS** |

The repository stayed clean through all six. Only the two untracked docs remain.

### Why mypy failed for Codex and passes here, proved not asserted

The first audit ran a user-site mypy against the system Pillow 10.2.0. Pillow did not ship a `py.typed` marker until 10.3.0:

```
$ pip download --no-deps Pillow==10.2.0 && unzip -l *.whl | grep py.typed  -> Pillow 10.2.0: py.typed ABSENT
$ pip download --no-deps Pillow==10.3.0 && unzip -l *.whl | grep py.typed  -> Pillow 10.3.0: py.typed PRESENT
$ pip download --no-deps Pillow==11.0.0 ...                                -> Pillow 11.0.0: py.typed PRESENT
```

**This is not "mypy is clean, move on."** `pyproject.toml` declares `dependencies = ["Pillow>=10.1"]`. That floor permits a resolver to install 10.1 or 10.2, in which case `mypy snitch` legitimately fails for a user following the README's own development instructions. The correct fix is to raise the floor to `Pillow>=10.3`. Adding `types-Pillow` would be suppression of a real packaging defect, and is the wrong call. Finding **P1-1**.

---

## 4 · Request-to-evidence matrix

Historical docs and commit messages were not accepted as implementation evidence anywhere in this table.

### Gemini Work Order 1 · Python engine and MCP server

| Request | State | Evidence |
| --- | --- | --- |
| `snitch/mcp.py` | **ABSENT** | `ls snitch/mcp.py` -> No such file. Repo-wide grep for "mcp" over `*.py`/`*.toml`/`*.md` outside `docs/` returns zero hits. |
| `tests/test_mcp.py` | **ABSENT** | `ls tests/test_mcp.py` -> No such file. |
| `snitch_inspect` | **ABSENT as MCP; core exists** | `core.inspect()` returns camera, gps, credit, c2pa, c2pa_status, c2pa_error, ai, has_any_credit. Adapter work only. |
| `snitch_strip_metadata` | **ABSENT as MCP; core exists** | `core.strip()` at `snitch/core.py:369`. JPEG and PNG only, see P1-3. |
| `snitch_add_credit` | **ABSENT as MCP; core exists** | `core.write_credit()`. Mutates in place, so an adapter needs explicit output handling. |
| `snitch_verify_c2pa` | **ABSENT as MCP; core exists** | `core.read_c2pa_report()` at `snitch/core.py:105`. |
| `snitch_clean_text` | **CONTRADICTED. No Python implementation of any kind.** | `grep -rn "200b\|200c\|200d\|FEFF\|zero.width\|clean_text\|sanitize" snitch/` returns **zero hits**. The entire text scanner lives only in the web app at `app/snitch/lib/text.ts` and `app/snitch/TextMark.tsx`. See P0-3. |
| `snitch-mcp` entry point | **ABSENT** | `[project.scripts]` declares only `snitch`, `no-comment`, `credit`. |
| `snitch mcp` subcommand | **CONTRADICTED by design law** | `snitch/cli.py:7-9`: "There is deliberately no umbrella command with these hidden underneath as subcommands. Each tool is the thing it is called, because the name IS the interface". See P2-1. |
| `pytest -v`, `mypy snitch`, `ruff check snitch` clean | **PRESENT** | Section 3. All pass in a clean venv. |
| README: `pip install snitch-tools` | **PRESENT AND FALSE** | `README.md:126`. `https://pypi.org/pypi/snitch-tools/json` -> **HTTP 404**. See P0-1. |
| README: MCP setup for Claude, Cursor, Antigravity | **ABSENT** | `grep -i "mcp\|claude desktop\|cursor\|antigravity" README.md` -> zero hits. |

### Gemini Work Order 2 · Web Studio upgrade

| Request | State | Evidence |
| --- | --- | --- |
| Five-workspace Studio with switcher | **CONTRADICTED by a standing decision** | `app/snitch/Panel.tsx:6` header reads "SNITCH · THE ONE-PANEL APP." Its only state is `type Mode = "photo" \| "text"` and `type Step = "read" \| "clean" \| "credit"` (`Panel.tsx:57-58`). `Panel.tsx:29-31` records Ken rejecting the alternative. See P2-2. |
| Real-time visual hash / canvas integrity proof | **ABSENT** | `grep -rn "subtle.digest"` over the whole repo excluding `node_modules` returns **zero hits**. The only SHA-256 in the app is HMAC for lead links (`app/lib/lead-links.server.ts:22`) and Helcim webhooks (`app/lib/helcim.ts:105`). See P1-2. |
| GitHub badge and release download links | **ABSENT, and cannot be built truthfully** | `grep -rn "github.com/3d3dcanada/snitch" app/ public/ workers/` -> zero hits. `GET /repos/3d3dcanada/snitch/releases` -> `[]`. `GET .../tags` -> `[]`. **There are no releases and no tags.** Building a download control would fabricate one. See P0-2. |
| One-click `pip install snitch-tools` copy | **ABSENT, and would publish a false claim** | `grep -rn "pip install" app/ public/ workers/` -> zero hits. Shipping it would put a command that 404s on the public site. See P0-1. |
| MCP config snippet modal | **ABSENT** | `grep -rn "mcpServers" app/ public/ workers/` -> zero hits. Nothing to configure until P0-3 and the Python floor decision are settled. |
| `fetchRemoteManifests: false` strictly set | **ALREADY SATISFIED** | `app/snitch/Inspector.tsx:357-364` sets both `fetchRemoteManifests: false` and `settings.verify.remoteManifestFetch: false`, with self-hosted `wasmSrc` and `workerSrc`. It is the only C2PA reader in the app: `createC2pa` appears once, `createL2ManifestStore` never. |
| Intent routes go straight to the active tool | **ALREADY SATISFIED in substance** | See section 6. |

---

## 5 · Findings by severity

### P0 · Correctness and truth

**P0-1 · `pip install snitch-tools` is a live false instruction on a public repository.**
`README.md:126` instructs `pip install snitch-tools`. PyPI returns HTTP 404 for that name. The repository is public (`GET /repos/3d3dcanada/snitch` unauthenticated -> `"private": false, "visibility": "public"`, pushed 2026-08-25), so anyone who finds it is handed a command that cannot work. Gemini's Work Order 2 asks to copy this same command onto 3d3d.ca, which would multiply the false claim onto the live site. Either publish to PyPI or change the README to the install method that actually works today.

**P0-2 · There are no GitHub releases or tags, so a release download control would be fabricated.**
`GET /repos/3d3dcanada/snitch/releases` returns `[]`, `GET .../tags` returns `[]`. The Codex work order already warned against a fake download control. This confirms the warning with live data. Do not build the control. Cut a release first, or drop the item.

**P0-3 · `snitch_clean_text` is net-new business logic, not an adapter, and the DoD forbids that shape.**
The Python package has no text handling of any kind. Zero hits for zero-width, ZWNJ, ZWJ, BOM, `clean_text` or `sanitize` across `snitch/`. Gemini's proposal and its own Claim B verification both cite `app/snitch/lib/text.ts` and `TextMark.tsx`, which are TypeScript in a different repository. The Codex DoD says "Build a thin adapter over tested core behavior, not parallel business logic". `snitch_clean_text` cannot satisfy that today. It is either a port of `text.ts` into Python with the full emoji-ZWJ, Arabic, Persian, Kurdish and Indic ZWNJ test matrix, which is its own work order, or it is dropped from the MCP surface. This is an owner decision, not an implementation detail.

### P1 · Packaging, support and integrity

**P1-1 · The declared Pillow floor permits an install where the project's own type gate fails.**
`Pillow>=10.1`, but `py.typed` first shipped in Pillow 10.3.0. Proved in section 3 by downloading and inspecting the 10.2.0, 10.3.0 and 11.0.0 wheels. Raise the floor to `Pillow>=10.3`. Do not add `types-Pillow`.

**P1-2 · The web integrity claim is offline-verified, never per-run, and the UI does not say so.**
`app/snitch/Scrub.tsx:426-429` renders `pixels untouched, nothing re-compressed` after every strip. The runtime does exactly three checks: a magic-byte sniff (`Scrub.tsx:243`), a JPEG EOI marker check (`Scrub.tsx:142-146`), and a byte-length delta. There is no decode, no re-decode comparison and no hash. The claim rests on a source comment, `Scrub.tsx:34-36`: "Verified 2026-08-26 against real fixtures for all five formats". That is a legitimate basis for the algorithm, but the sentence renders to a user as though this file, this run, was checked. The Python side does verify per run: `core.pixels_identical()` compares decoded-pixel SHA-256, which is why the CLI can print `pixels byte-identical` truthfully. The web and CLI make the same claim on different evidence. If the visual hash from Gemini's Work Order 2 is built, it must label algorithm, input, limitation and result, and must never conflate a canvas digest with a file hash, C2PA validity or identity.

**P1-3 · Python and web support different formats, and nothing in the MCP proposal accounts for it.**
Python `core.strip()` handles JPEG and PNG only, and refuses the rest honestly:
```
$ no-comment t.webp
  t.webp: .webp is not supported for lossless stripping. Only JPEG and PNG.
  EXIT=1
```
The web `strip.ts:51` handles `"jpeg" | "png" | "webp" | "heic" | "avif"`. Any `snitch_strip_metadata` tool description and any README must state the JPEG-and-PNG limit, or MCP clients will be told the tool can do things it cannot.

**P1-4 · The official MCP Python SDK floor is confirmed incompatible with the declared floor.**
Checked live, not from memory:
```
$ curl -s https://pypi.org/pypi/mcp/json | ...
version 2.1.1 · requires_python >=3.10
```
`pyproject.toml` declares `requires-python = ">=3.9"`. The conflict is real and the owner decision the work order describes still stands unmade. This is the single gate that blocks all MCP work.

### P2 · Design, UX and reporting

**P2-1 · `snitch mcp` would break the package's own stated design law.**
`snitch/cli.py:7-9` states there is deliberately no umbrella command with subcommands hidden underneath. `snitch-mcp` as a separate console script is compatible with that law. `snitch mcp` is not. Recommend `snitch-mcp` only.

**P2-2 · The Studio proposal reopens a decision Ken already made.**
`Panel.tsx:6` is titled "THE ONE-PANEL APP", and the one-panel form was adopted after Ken rejected a stacked page of separate tools. `docs/HANDOFF-2026-08-26-SNITCH-AND-NEXT.md:168-170` adds a binding constraint: the standalone tools must keep their own file picker, because nine intent pages rely on it, and "Do not remove that fallback." A five-workspace Studio is a redesign, not a hardening item. It needs Ken, not an agent.

**P2-3 · The CLI knows the image was altered and does not say so.**
On a tampered signed file the CLI prints `C2PA Content Credential  [Invalid]` and nothing more specific, while the structured data it already holds says exactly why:
```json
{"code": "assertion.dataHash.mismatch",
 "explanation": "asset hash error, name: jumbf manifest, error: hash verification( Hashes do not match )"}
```
"Invalid" is honest but uninformative. The user cannot tell "the pixels were changed after signing" from "the signature is malformed". The data is there. Surfacing it is a small, high-value change.

---

## 6 · Privacy, C2PA, routing and integrity findings from live observation

### Live browser evidence, Ken's Brave via claude-in-chrome, 2026-08-29

`https://3d3d.ca/snitch` rendered correctly and matches source: one-panel app, Photo and Text chooser, hero `FREE · NO ACCOUNT · NOTHING IS UPLOADED`, H1 "Your files are telling on you".

*A note on a non-finding.* My first screenshot showed the page completely unstyled. A reload rendered it correctly. That was a transient capture before CSS applied, not a defect, and it is recorded here only so it is not mistaken for one later.

**Resource waterfall on page load:**

```
totalResources: 43
hosts: { "3d3d.ca": 40, "static.cloudflareinsights.com": 2, "3d3d-analytics.3d3dcanada.workers.dev": 1 }
c2pa assets loaded: []          <- none, the 6.2 MB SDK is lazy and did not load
navigator.globalPrivacyControl: true
```

**LIVE-1 · Cloudflare edge-injects a third-party beacon that the site's own CSP then blocks.**
The served HTML contains `<script src="https://static.cloudflareinsights.com/beacon.min.js/..." data-cf-beacon ...>`. It is not in the app source: `grep -rn "cloudflareinsights\|cf-beacon" app/ workers/` finds it only inside the CSP string. It is injected at the edge by Cloudflare Web Analytics automatic setup.

Cloudflare injects it only for requests that accept HTML, which is why a plain `curl` misses it and a browser-shaped one does not:
```
$ curl -s https://3d3d.ca/snitch | grep -c cloudflareinsights                          -> 0
$ curl -s -H 'Accept: text/html' -H 'User-Agent: Mozilla/5.0 ...' ... | grep -c ...    -> 1   (also on /)
```

**It does not actually execute.** Measured in the page, not inferred:
```
static.cloudflareinsights.com/beacon.min.js  encodedBodySize: 0  transferSize: 0  duration: 0
cdn-cgi/rum POSTs: []
```
The live CSP `script-src` is `'self' 'unsafe-inline' 'wasm-unsafe-eval' https://challenges.cloudflare.com https://news.google.com`. `static.cloudflareinsights.com` is not on it, so the browser blocks the load and no third-party analytics traffic leaves.

The current outcome is correct. The mechanism is accidental. The tag is still injected on every page, and the CSP `connect-src` already permits `https://cloudflareinsights.com`. The day anyone widens `script-src`, or Cloudflare moves the beacon to an allowed host, a third-party tracker starts firing on the one page whose entire promise is privacy, with no consent gate in front of it. The durable fix is to turn Cloudflare Web Analytics automatic injection off for the zone so the tag stops being injected at all. **This belongs to the agent that owns 3d3d-site, not to SNITCH.**

**LIVE-2 · The first-party analytics pixel fires with Global Privacy Control set to true.**
Observed: `3d3d-analytics.3d3dcanada.workers.dev/hit` in the resource list, with `navigator.globalPrivacyControl === true`.

This is deliberate and documented. `app/lib/analytics.ts:338-341` gates the pixel on base-set and not-localhost only, outside the consent and DNT machinery that gates `track()`. The payload is host, path, referrer host and a cache-busting timestamp. No file data.

The `track()` path, which carries the `snitch_tool` events, is correctly gated: `browserSaysNo()` at `analytics.ts:142-147` checks `doNotTrack` and `globalPrivacyControl`, and `consentAllows()` at `:156-162` holds events until the edge reports a region. The event contract at `analytics.ts:47-56` is explicit that a SNITCH event may carry only counts and outcomes, never "a filename, size, hash, dimension, coordinate, EXIF value, or anything typed into a credit or marker field". I read all 20 `snitch_tool` call sites and every payload obeys it.

So the product promise is intact: **no file data leaves the browser.** But the page invites scrutiny, the source comment says so in as many words, and a privacy-minded reader watching the network tab with GPC on will see an off-site request they cannot suppress. Given that Reddit and r/privacy are on the distribution plan, this is the most attackable surface SNITCH has. It is a measurement decision for Ken, not a bug.

**LIVE-3 · Feedback transmits visitor free text through the analytics path.**
`app/snitch/Feedback.tsx:25-29` sends `{ tool, answer, note: note.trim().slice(0, 240) }` where `note` is typed by the visitor. It uses the gated `track()` path and the UI promises, truthfully, "This sends nothing about your file. Not its name, contents, metadata, hash or location." The promise is about the file and it is kept. Worth knowing that free text does traverse the same worker.

**Copy verdict.** The live wording is correct. `curl` on `/snitch` finds `nothing is uploaded` four times and `never uploaded` once, and finds zero instances of "no data is sent", "zero server", "no network requests" or "nothing is sent". The Codex correction has been applied and must stay applied.

### Routing: Gemini's request is already satisfied in substance

Verified live and in source:

| URL | Live | Renders |
| --- | --- | --- |
| `/remove-exif-online` | HTTP 200 | `<Scrub />`, `app/routes.ts:129` -> `snitch-intent.tsx`, `intent-data.ts:95` `component: "scrub"` |
| `/add-copyright-to-photo` | HTTP 200 | `<Credit />`, `intent-data.ts:185` `component: "credit"` |
| `/pdf-metadata-remover` | HTTP 301 -> `/does-a-pdf-show-who-made-it` | `<Documents />`, `intent-data.ts:248`; redirect from `RENAMED_SLUGS` at `intent-data.ts:336-338`, emitted by the loader at `snitch-intent.tsx:37-39` |

`curl` on `/remove-exif-online` server-renders the tool's own copy, so the SSR requirement holds.

The literal reading in the Codex work order is correct: `snitch-intent.tsx:29-32` lazy-imports `Inspector`, `Scrub`, `Credit` and `Documents` directly and never imports `Panel`, so these routes do not enter an "active Panel state". **The user-visible outcome Gemini asked for already happens.** The visitor lands on the page and the correct tool is right there with its own file picker. Rewiring these routes through `Panel` would remove the standalone-picker fallback that `HANDOFF-2026-08-26-SNITCH-AND-NEXT.md:168-170` explicitly forbids removing. **Recommendation: close this item as already satisfied. Do not build it.**

### C2PA: the implementation is better than either prior document recorded

Every result below is a real `c2patool 0.27.15` validation against a fixture I signed with the tool itself.

| Case | Observed |
| --- | --- |
| Signed, self-signed cert | `C2PA Content Credential  [Valid]` plus `SIGNER IDENTITY UNTRUSTED (certificate not on validator trust list)` and `asserts a camera made this, not a model`. JSON: `c2pa_status: present`, `validation_status: [{"code": "signingCredential.untrusted"}]` |
| One byte flipped in the scan data | `[Invalid]`. JSON adds `assertion.dataHash.mismatch`. `credit --verify` exits **1**. |
| c2patool removed from PATH and `~/.cargo` | `C2PA Content Credential detected; validation unavailable` / `Install c2patool to read and validate it.` JSON: `c2pa_status: "detected-unverified"` |
| No manifest | `no C2PA Content Credential`, `c2pa_status: "absent"` |

**Correction to the work order.** It records four states: `present`, `absent`, `unavailable`, `error`. There is a fifth, and it is the best one: **`detected-unverified`**. When c2patool is missing, SNITCH still detects the JUMBF container through ExifTool and says a credential exists but cannot be validated, rather than falling back to "absent". That distinction is exactly the failure mode that misleads people, and it is already handled. Preserve it in any MCP adapter. The README documents it correctly at lines 138-141.

`snitch/sign.py:8-13` is explicit in the module docstring that generated certificates are self-signed, development-grade, not on the C2PA trust list, and that strict validators will report the signer as unknown. The CLI repeats it in output. Nothing here claims a self-signed credential proves identity.

**A gap of the same family, on the other side.** `snitch` reports nothing at all about PNG text chunks. Fixture: a PNG carrying `parameters` (a full Stable Diffusion prompt), `workflow` (a ComfyUI graph) and an `iTXt` description.

```
$ snitch gen.png
  gen.png  313 bytes
    no location data
    NO CREDIT AT ALL
    no C2PA Content Credential

$ snitch --json gen.png     ->  "camera": {}, "gps": {}, "credit": {}, "ai": null

$ exiftool -G gen.png
  [PNG] Parameters  : masterpiece, (ugly hands:1.4).Steps: 30, Sampler: Euler a
  [PNG] Workflow    : {"nodes":[{"id":1}]}
  [PNG] Description : cafe naive zhongwen (emoji)
```

ExifTool sees the generation prompt. SNITCH does not report it, and `ai` is `null` because AI detection reads only the C2PA `digitalSourceType` (`core.py:174-185`). `no-comment` removes these chunks correctly, so Gemini's Claim A is right that stripping is handled. But the tool is named for telling you what your file is saying, and on the single most common AI-image metadata carrier it says nothing. Finding **P1-5**, and it is the most product-damaging gap in this report.

---

## 7 · Fixture matrix: what was exercised and what happened

Built from a clean wheel install (`pip install dist/snitch_tools-0.1.0-py3-none-any.whl`) so this is installed-artifact behaviour, not source-tree behaviour.

| Case | Result |
| --- | --- |
| JPEG with EXIF, GPS, IPTC, XMP, Unicode, comment | `snitch` prints `LOCATION IS IN THIS FILE`, the coordinates, camera, and credit including `Renée Åberg` and `café naïve 中文 😀`. Unicode round-trips correctly. |
| Strip JPEG | `removed 3,486 bytes of metadata  pixels byte-identical`. Output carries zero metadata. |
| Strip PNG | `removed 173 bytes of metadata  pixels byte-identical`. All text chunks gone. |
| **Source preservation** | Source SHA-256 identical before and after, both formats. |
| **Symlink** | `no-comment link.jpg` reads through the link and writes a new regular file `link-clean.jpg`. It does not write through the symlink and does not touch the target. Safe. |
| **Option-like filename** | A real file named `--force.jpg` is inspected and stripped correctly, and the tool's own remediation hint emits the `--` guard: `no-comment -- --force.jpg`. |
| **Malformed file** | `broken.jpg: not an image (ExifTool identified TXT)` / `broken.jpg: not a JPEG`. No traceback. Exit 1. |
| **Non-file input** | `adir: not a regular file`, `/dev/null: not a regular file`. Exit 1. |
| **Output collision** | `coll-clean.jpg: output exists; pass --force to replace it`. Exit 1. Existing file untouched, still 0 bytes. |
| **Unsupported format** | `t.webp: .webp is not supported for lossless stripping. Only JPEG and PNG.` Exit 1. |
| **`credit` re-encode check** | Decoded-pixel SHA-256 identical before and after signing. `credit` does not re-encode. |
| **`credit` GPS default** | GPS dropped by default. `--keep-gps` is opt-in. Correct doxxing default. |
| **`--sign` without `--digital-source`** | Refused, exit 2: "`--sign requires --digital-source so the credential does not guess`". |
| **Exit codes** | Every failure path returns 1 (or 2 for argparse). Verified without a pipe, since piping to `head` masks the real status. |

No partial artifacts were left behind after any failure.

---

## 8 · Evidence gaps, stated plainly

1. **`npm run typecheck` and `npm run build` in `/home/wess/3d3d-site` were not run this pass**, by deliberate abstention: another agent session owns that checkout. Section 2 gives the reasoning. All web findings above are source proof plus live-site proof, never build proof.
2. **No file was pushed through the live web tools.** Inspector, Scrub, Credit, Documents and Text Scanner were not exercised with a real file in the browser, so the C2PA remote-manifest opt-out is proved from source and from the fact that no C2PA asset loads on page view, and **not** from an observed waterfall during an actual inspect. That test requires driving a file through the live tool, which would have meant working over another agent's live deploy.
3. **No 390px pass.** `AGENTS.md:90-91` requires both viewports for any UI claim. I make no UI-quality claim here, only behavioural ones.
4. **No platform round-trip testing.** Every row of the survival table is still, correctly, marked unverified inference. `snitch/survival.py:6` says so itself, and `RESEARCHED = "2026-08-25"` dates it.
5. **The `mcp` SDK was never installed or run**, so no MCP client session, tool schema, or stdout-purity claim exists in either direction.
6. **Method note.** `AGENTS.md:184` reads "One focused effort, no subagent fan-out, ever." I used one read-only Explore agent to gather file-and-line evidence in `/home/wess/3d3d-site`. It edited nothing. Recording it because the rule exists and I want the deviation on the page rather than buried.

---

## 9 · Owner decisions required before any implementation

Nothing in section 10 can start until these are answered. They are Ken's, not an agent's.

1. **Python floor.** The official `mcp` SDK is 2.1.1 and requires `>=3.10`. SNITCH declares `>=3.9`. Choose: (a) raise SNITCH to 3.10+ and make MCP a first-class part of the package, (b) keep 3.9 and defer MCP entirely, or (c) ship MCP as a separately installed 3.10+ extra. Do not pin a stale SDK to keep 3.9.
2. **`snitch_clean_text`.** Port `text.ts` to Python with the full script-safety test matrix, which is real work in its own right, or drop it from the MCP surface. See P0-3.
3. **PyPI.** Publish `snitch-tools`, or change the README. The current state is a false instruction on a public repo. See P0-1.
4. **The analytics pixel on a privacy tool.** Leave it outside the consent gate, or gate it. See LIVE-2. This is a measurement decision with a real cost either way.
5. **The Studio.** The one-panel app was chosen after you rejected the alternative. Gemini's five-workspace Studio reopens that. Yes or no. See P2-2.

---

## 10 · The single safe next work order

Per the work order's instruction to end with exactly one, and per the standing method, this is **WO-S01** and it is the only one that needs no decision from section 9.

### WO-S01 · Tell the truth about install, and report what PNG chunks are saying

**Why this one.** It is entirely inside `/home/wess/snitch`, which is clean, idle and owned by no other session. It touches no design decision, needs no Python floor answer, and does not go near `/home/wess/3d3d-site` while another agent holds it. It closes the two findings that are actively misleading real users right now.

**Scope, and nothing beyond it:**

1. **P0-1.** Fix `README.md:126`. Either publish `snitch-tools` to PyPI first and keep the line, or replace it with the install that works today. No other README edit.
2. **P1-1.** `pyproject.toml`: `Pillow>=10.1` becomes `Pillow>=10.3`. Add a one-line comment giving the reason, which is `py.typed`.
3. **P1-5.** `core.inspect()` reports PNG `tEXt`, `zTXt` and `iTXt` chunks in a new field, and the human renderer surfaces a generation prompt or workflow when one is present. When such a chunk is found, `ai` reports generative provenance from the chunk as well as from C2PA, with the source of the signal named so a chunk-derived answer is never presented as a C2PA-validated one.
4. **P2-3.** When `validation_status` contains `assertion.dataHash.mismatch`, the CLI says the image was altered after signing, instead of only `[Invalid]`.

**Definition of done:**

- New tests covering: a ComfyUI-style PNG with `parameters` and `workflow`, an `iTXt` chunk with non-ASCII text, a PNG with no text chunks, and a tampered signed JPEG asserting the altered-since-signed message. Written before the change where practical.
- `pytest -q`, `ruff check snitch tests`, `mypy snitch`, `python -m build` and `twine check dist/*` all pass **in a fresh venv built with `pip install -e '.[dev]'`**, never against system site-packages. Paste the real output.
- A wheel built from the change, installed clean, and the four behaviours exercised through the installed console scripts, with output pasted.
- Confirm `no-comment` still strips those chunks and still reports `pixels byte-identical`, so the new reporting has not disturbed the strip path.
- The repository is left uncommitted unless Ken says otherwise. No push, no publish, no PyPI upload without a separate explicit yes.

**Out of scope, stated so it is not quietly picked up:** any MCP file, any `pyproject.toml` script entry, any Python version change, any text-cleaning code, and anything at all inside `/home/wess/3d3d-site`.

The rest of the route to finished is laid out in `docs/work-orders/BOARD-SNITCH-FINISH-2026-08-29.md`. Nothing on that board past WO-S01 may start before section 9 is answered.

---

## 11 · Things that must never be claimed, carried forward

Restated because they survived this audit and are now backed by measurement.

- Never "zero server data transfer", "no network requests" or "nothing is sent". The correct claim is "your file stays on this device" or "no file is uploaded". Two off-site requests occur on page load, and one of them fires with GPC set.
- Never present a self-signed credential as verified identity. `[Valid]` means the asset binding holds. `SIGNER IDENTITY UNTRUSTED` is a separate axis and both must be shown.
- Never read a missing C2PA manifest as evidence that an image is not AI-generated. `detected-unverified` exists precisely so that "we could not check" never collapses into "there is nothing there".
- Never claim metadata stripping touches an in-pixel watermark. SynthID and its relatives live in image data.
- Never call a canvas digest a file hash, a strip proof, a C2PA validity result, or a proof of identity.
- Never call an unrun gate clean. The mypy result in the first audit was real for the environment it ran in, and wrong about the code. Say which environment.
