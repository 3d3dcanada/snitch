# SNITCH · Board to finished

> **SUPERSEDED IN PART, 2026-08-30.** Ken: *"This needs to be built in Rust, period."* The Python
> was the wrong language for this machine and nobody, including this board, had said so. Everything
> below that concerned the Python package is now history: the tool is Rust, at the root of the
> repository, and the Python is kept verbatim in `legacy/python/` as the specification the port is
> checked against. What survives unchanged is the web-side work, which was never in this repo, and
> the truth findings, which the Rust inherits.
>
> The port is `docs/RUST-PORT-2026-08-30.md` and pull request #1.

**Opened:** 2026-08-29
**Author:** Claude, from `docs/AUDIT-CLAUDE-SNITCH-2026-08-29.md`
**Method:** one WO per sitting, lowest open number first, status row set and evidence pasted before anything is called DONE.

Read the audit first. Every claim below traces to a numbered finding in it. Nothing here is a plan built on a prior document's say-so.

---

## Status

| WO | Title | Blocked by | Status |
| --- | --- | --- | --- |
| S01 | Truthful install, and report PNG text chunks | nothing | **DONE 2026-08-30** |
| S02 | Python floor and MCP shape | decision taken below | **DONE 2026-08-30** |
| S03 | `snitch-mcp` stdio server | S02 | **DONE 2026-08-30**, five tools not four |
| S04 | Port the text scanner to Python | S02 | **DONE 2026-08-30** |
| S05 | Publish `snitch-tools` and cut a release | Ken, decision 3 | **OPEN, needs his yes** |
| S06 | README with host-verified MCP setup | S03 | **DONE 2026-08-30** |
| S07 | Web developer hub | S05, and the 3d3d-site owner | BLOCKED |
| S08 | Per-run integrity proof in the web Scrub | 3d3d-site owner | BLOCKED |
| S09 | Turn off Cloudflare beacon injection | 3d3d-site owner | HANDOFF, not ours |
| S10 | Platform round-trips | Ken's own forty minutes | OPEN, his time |

Everything inside `/home/wess/snitch` is finished and proved. What is left needs either Ken, because
publishing is outward-facing and irreversible, or the agent that owns `/home/wess/3d3d-site`.

**The working tree is uncommitted.** Nothing was committed, pushed or published.

### Gates, in a fresh venv from `pip install -e '.[dev]'`

```
pytest -q                    101 passed
ruff check snitch tests      All checks passed!
mypy snitch                  Success: no issues found in 8 source files
python -m build              Successfully built snitch_tools-0.1.0.tar.gz and .whl
twine check dist/*           both PASSED
```

And with the SDK absent, which is what the CI 3.9 job sees:

```
mypy snitch                  Success: no issues found in 8 source files
pytest -q                    90 passed, 1 skipped (tests/test_mcp.py: the MCP SDK is not installed)
```

---

## Decisions

### 1 and 2, taken under a stated assumption. Both reverse cheaply if either is wrong.

**Python floor: the extra, option (c).** `mcp` 2.1.1 requires `>=3.10`, SNITCH declares `>=3.9`.
Raising the whole package to 3.10 would drop users from three commands that work perfectly well on
3.9, in order to gain one optional surface. So `requires-python` is untouched and MCP ships as
`pip install "snitch-tools[mcp]"`, which pip refuses to resolve on 3.9 of its own accord.
`snitch-mcp` on a machine without the SDK prints what to install and exits 1, rather than throwing
a traceback. Nobody is dropped, and it reverses with one line if you would rather raise the floor.

**`snitch_clean_text`: ported, not dropped.** `app/snitch/lib/text.ts` is now `snitch/text.py`,
behaviourally identical, with the script-safety matrix the port demanded. That makes it five MCP
tools rather than four, and it means the CLI, the MCP server and the website can no longer disagree
about what a piece of text contains.

### Still yours

3. **PyPI.** The README now points at the git install, which is verified working, so nothing false
   ships. Publishing `snitch-tools` to PyPI is outward-facing and effectively irreversible, so it
   waits for your explicit yes. That is S05.
4. **The analytics pixel.** Leave it outside the consent gate on a privacy tool, or gate it.
   Measured live: it fires with GPC set. Audit finding LIVE-2.
5. **The Studio.** The one-panel app was chosen after you rejected the alternative. Gemini wants to
   reopen it. Audit finding P2-2.

---

## WO-S01 · Truthful install, and report PNG text chunks

**Status:** DONE 2026-08-30
**Repo:** `/home/wess/snitch` only. `/home/wess/3d3d-site` was not touched.
**Closes:** P0-1, P1-1, P1-5, P2-3

The full scope, DoD and out-of-scope list are in section 10 of the audit. In short:

1. `README.md:126` stops instructing an install that 404s.
2. `Pillow>=10.1` becomes `Pillow>=10.3`, because `py.typed` first shipped in 10.3.0.
3. `core.inspect()` reports PNG `tEXt`, `zTXt` and `iTXt`, and the renderer surfaces a generation prompt when one is there. `ai` names the source of its signal so a chunk-derived answer never reads as a C2PA-validated one.
4. `assertion.dataHash.mismatch` renders as "altered after signing", not bare `[Invalid]`.

**DoD:** new tests for all four, then `pytest -q`, `ruff check snitch tests`, `mypy snitch`, `python -m build`, `twine check dist/*`, every one of them in a fresh venv from `pip install -e '.[dev]'`, output pasted. Then a wheel built, installed clean, and the behaviours exercised through the installed scripts. Confirm `no-comment` still reports `pixels byte-identical`.

---

## WO-S02 · Python floor and MCP shape

**Status:** DONE 2026-08-30
**Closes:** P1-4, and unblocked everything MCP

Not a coding job. Record the answer, then make `pyproject.toml` say it: `requires-python`, the `mcp` dependency or extra, and the `target-version` in the ruff and mypy config. If the answer is "defer MCP", write that down and close S03, S04 and S06 as declined rather than leaving them open forever.

**Constraint that survives whatever is chosen:** `snitch-mcp` as a console script only. Not `snitch mcp`. `snitch/cli.py:7-9` states there is deliberately no umbrella command with subcommands underneath, and that law predates this proposal.

**DoD:** the decision written into `pyproject.toml` and the README, all five gates still green, and this board's status table updated.

---

## WO-S03 · `snitch-mcp` stdio server

**Status:** DONE 2026-08-30
**Ships:** `snitch_inspect`, `snitch_strip_metadata`, `snitch_add_credit`, `snitch_verify_c2pa`,
and `snitch_clean_text` once S04 made it possible

Four tools, not five. `snitch_clean_text` is S04 and only exists if decision 2 says so.

**Hard requirements, each one paid for by a finding in the audit:**

- A thin adapter over `core`. If CLI-only safety logic has to move, it moves into a public service layer in `core`, it is not reimplemented.
- **stdout is protocol wire.** No `print`, no banner, no ANSI, no subprocess noise before or during startup. Log to stderr only. `core._run` shells out to ExifTool and c2patool, so audit every path for leakage into stdout.
- Refuse non-regular files. Resolve output paths safely. Never overwrite the source. Keep the atomic sibling write, the collision check and `--force` semantics that the CLI already has and that the fixture matrix already proves work.
- Mutation tools return output path, format, byte delta, removed classes and an honest proof state. Never file bytes. Never a silent in-place mutation. `core.write_credit()` mutates the given file, so the adapter must handle output explicitly.
- Stamping is a separate, explicit affordance, because it repaints pixels and invalidates C2PA.
- **Tool descriptions state the JPEG and PNG limit.** P1-3. The web handles five formats, Python handles two. An MCP client told otherwise will hand users a failure.
- **Preserve all five C2PA states**, including `detected-unverified`. Collapsing "we could not check" into "absent" is the exact failure this tool exists to prevent.

**DoD:** discovery, schemas, each tool's success path, a malformed request, and c2patool-unavailable, all exercised **through a real MCP client session**, not a unit test that imports the handler. Every server process stopped in the same sitting. `tests/test_mcp.py` present and green. All five gates green.

---

## WO-S04 · Port the text scanner to Python

**Status:** DONE 2026-08-30
**Closes:** P0-3

Kept, and ported. The source is `app/snitch/lib/text.ts` and `app/snitch/TextMark.tsx` in the 3d3d-site repo. Copy the behaviour exactly, per the standing law on old code.

The whole difficulty is one thing: stripping `‌` and `‍` indiscriminately destroys emoji ZWJ sequences and breaks legitimate Arabic, Persian, Kurdish and Indic orthography. The TypeScript already solves this. The port must carry the test matrix that proves it, and non-ASCII is never treated as suspicious on its own.

**DoD:** a test matrix covering emoji ZWJ sequences, Arabic and Persian and Kurdish ZWNJ, Indic ZWJ, real tracking characters, bidi overrides, and odd whitespace. Documented Python behaviour. Five gates green.

---

## WO-S05 · Publish and release

**Status:** OPEN. The one thing here that needs Ken.
**Closes:** P0-1 completely, and P0-2

`snitch-tools` on PyPI returns 404 today. `/repos/3d3dcanada/snitch/releases` and `/tags` both return `[]`. Until both change, no download button and no `pip install` line may appear on the public site.

**DoD:** the PyPI page resolves, `pip install snitch-tools` works in a clean venv on a machine that is not this one, and a tagged GitHub release exists. Requires Ken's explicit yes to publish. Publishing is not covered by any standing authorization.

---

## WO-S06 · README, host-verified

**Status:** DONE 2026-08-30

Setup for Claude, Cursor and Antigravity, each snippet verified against that host's **current official documentation**. Never invent a schema. If a host's format cannot be verified, it does not go in.

States truthfully: the Python floor, that MCP is or is not included, that ExifTool is required, that c2patool is required for signing and full validation, and that lossless stripping is JPEG and PNG only.

**DoD:** every URL resolves. Every snippet traced to the official doc it came from, cited in the WO.

---

## WO-S07 · Web developer hub

**Status:** BLOCKED on S05 and on the 3d3d-site owner

Gemini's Work Order 2 item 2. Cannot be built truthfully before S05: the install command 404s and there is no release to download. Install commands and config snippets are documentation, not proof of installation. No secret, no network server, no invented host schema.

---

## WO-S08 · Per-run integrity proof in the web Scrub

**Status:** BLOCKED on the 3d3d-site owner
**Closes:** P1-2

`Scrub.tsx:426-429` renders `pixels untouched, nothing re-compressed` after every run, on the strength of an offline verification recorded in a source comment. The runtime does a magic-byte sniff, a JPEG EOI check and a byte-length delta. It does not decode, hash or compare.

If this is built, it must label algorithm, input, limitation and result, and give each format an honest state: "verified decoded pixels identical", "container guarantee, not decoded in this browser", or "not verified". Add a deliberately changed-pixel fixture that fails. A green badge must never become a generic trust signal.

Note that the Python side already does this honestly per run through `core.pixels_identical()`, which is why the CLI may say `pixels byte-identical` and the web currently may not.

---

## WO-S09 · Cloudflare beacon injection · HANDOFF

**Status:** not SNITCH's to fix. Give it to whoever owns 3d3d.ca.
**Source:** audit finding LIVE-1

Cloudflare Web Analytics automatic setup injects `static.cloudflareinsights.com/beacon.min.js` into every HTML response. It is not in the app source. The site's own CSP `script-src` does not list that host, so the browser blocks it and it never executes: measured `transferSize: 0`, `encodedBodySize: 0`, no `cdn-cgi/rum` POST.

The outcome is right, the mechanism is accidental. The tag is still injected on every page and `connect-src` already permits `https://cloudflareinsights.com`. Widen `script-src` once, or let Cloudflare move the beacon to an already-allowed host, and a third-party tracker starts firing on the privacy tool with nothing gating it.

**Fix:** turn Cloudflare Web Analytics automatic injection off for the zone so the tag stops being injected. One dashboard setting, not a code change.

---

## WO-S10 · Platform round-trips

**Status:** OPEN, Ken's own time

Every row of the survival table is marked unverified inference, honestly, at `snitch/survival.py:6`, dated `RESEARCHED = "2026-08-25"`. One test file through each platform turns the table into the only measured data in this niche. An agent cannot do this: it needs real accounts and real uploads.

---

## Closed on arrival

Two of Gemini's items are already done and must not be rebuilt.

**Remote manifests are already disabled.** `Inspector.tsx:357-364` sets both `fetchRemoteManifests: false` and `settings.verify.remoteManifestFetch: false`, with self-hosted wasm and worker. It is the only C2PA reader in the app.

**The intent routes already land on the right tool.** `/remove-exif-online` serves 200 and renders `<Scrub />`. `/add-copyright-to-photo` serves 200 and renders `<Credit />`. `/pdf-metadata-remover` 301s to `/does-a-pdf-show-who-made-it`, which renders `<Documents />`. They mount the standalone tools rather than `Panel`, which is what `HANDOFF-2026-08-26-SNITCH-AND-NEXT.md:168-170` requires: those tools keep their own picker because nine intent pages depend on it, and that fallback must not be removed. Rewiring them through `Panel` would be a regression. **Closed as already satisfied.**

---

## Carried, from the 3d3d-site side

Not this board's work, but it is open and it is about SNITCH, so it should not go missing:

- The `/snitch` footer defect. `docs/HANDOFF-CODEX-2026-08-27.md:173-174`, Ken: "our footer looks like shit on that page." `CODEX-REPORT-2026-08-27.md:151-153` says E4 was deliberately left uninspected because the owning file was off limits. Still open, still needs its owner.
- `docs/AUDIT-1-CODEX-FINDINGS.md`, 19 findings with 4 critical, has no completion report. `docs/SNITCH-FIX-REPORT.md`, the deliverable named in `WO-SNITCH-FIX-PASS.md:35`, does not exist. Verify each finding against current code before acting: a prior session found two of three findings in `docs/AUDIT-2-FINDINGS.md` to be plainly wrong when checked.


---

## Evidence · what was built on 2026-08-30

Every command below was run in a disposable venv, never against system site-packages. The gate
output is in the Status section at the top of this board.

### The four S01 fixes

| Fix | Proof |
| --- | --- |
| `README.md:126` no longer says `pip install snitch-tools` | `pip install git+https://github.com/3d3dcanada/snitch.git` installed clean into a fresh venv and produced working `snitch`, `no-comment` and `credit` scripts. The PyPI name still returns HTTP 404, and no longer appears in the README. |
| `Pillow>=10.1` raised to `>=10.3` | `pip install snitch_tools-0.1.0.whl "Pillow==10.2.0"` now fails: `ResolutionImpossible`. The install that made `mypy snitch` fail is no longer reachable. |
| `snitch` reports PNG text chunks | On the ComfyUI fixture the installed wheel now prints `TEXT IS EMBEDDED IN THIS PNG`, the `parameters` prompt, the `workflow` JSON, then `A GENERATOR WROTE THIS` followed by `This is plain text, not a signed credential. It proves nothing on its own.` JSON reports `ai: generative`, `ai_source: png-text-chunk`. |
| A tampered signature says why | `snitch tampered.jpg` and `credit --verify tampered.jpg` both print `ALTERED AFTER SIGNING: these pixels are not the ones that were signed`, and `--verify` exits 1. The valid signed file is unaffected: still `[Valid]`, no altered line. |

`no-comment` still removes those chunks and still reports `pixels byte-identical`, checked after
the change. A caption that merely mentions Stable Diffusion is not treated as generated, which has
a test.

### The MCP server, driven through a real client

`snitch-mcp` installed from the built wheel, spoken to over stdio by a real `ClientSession`:

```
server           : snitch 0.1.0
tools discovered : ['snitch_add_credit', 'snitch_clean_text', 'snitch_inspect',
                    'snitch_strip_metadata', 'snitch_verify_c2pa']
inspect gen.png  : ai=generative via=png-text-chunk keywords=['parameters', 'workflow']
verify signed    : manifest=present integrity=valid signer_trusted=no
verify tampered  : integrity=altered
                   The pixels are not the ones that were signed: this image was altered after signing.
strip plain.jpg  : removed=3486 classes=['location', 'camera', 'credit'] pixels_identical=True
clean_text       : removed=1 chars=['U+200B']
                   emoji+persian intact: True
overwrite source : True | output would overwrite the source. Give a different out_path.
```

Note the third line of `verify`: presence, integrity and signer trust come back as three separate
answers, so a self-signed certificate can never read as verified identity.

Guards that are tested rather than asserted: a symlink is refused outright, output may not be the
source, an existing file is not replaced without `force`, a missing directory is refused, and a
refusal arrives as a protocol error with a sentence in it rather than an unexpected-crash envelope.

`tests/test_mcp.py::test_nothing_but_protocol_ever_reaches_stdout` runs a whole session with stderr
captured to a real file and asserts the banner went to stderr, because one stray byte on stdout
corrupts the wire for every client and only a real session can catch it.

### The text port

13 tests in `tests/test_text.py`. The ones that matter are the ones that must NOT fire: emoji ZWJ
sequences (woman technologist, four-person family, rainbow flag), Persian `می‌خواهم` and
`نمی‌دانم`, Kurdish `دەب‌ێت`, Devanagari `क‍ष` and `क‌ष`, Bengali `বা‌ংলা`. All pass through
untouched, `removed == 0`. Ordinary non-ASCII is never called suspicious.

What does fire: ZWSP, BOM, word joiner, invisible separator and function application are found and
removed; a right-to-left override is `alarming`; a Cyrillic `а` inside `pаypal` is caught while a
wholly Cyrillic phrase is not. Odd spaces and curly quotes are reported and deliberately left in
the prose, because removing them silently would edit someone's writing.

### Refactor, so there is one implementation and not two

`outpath`, `same_file`, `temporary_sibling`, `strip_atomic`, `asset_altered` and
`identity_untrusted` moved from `cli.py` into `core.py` as the public service layer. `cli.py` now
delegates to them. The MCP adapter uses the same functions, so it inherits the atomic sibling write,
the pixel check and the collision refusal rather than reimplementing them more weakly.

### Host setup, each verified against that host's own current docs

| Host | Config | Source |
| --- | --- | --- |
| Claude Code | `.mcp.json`, `type: "stdio"` | <https://code.claude.com/docs/en/mcp> |
| Claude Desktop | `claude_desktop_config.json`, macOS and Windows only | <https://modelcontextprotocol.io/docs/develop/connect-local-servers> |
| Cursor | `.cursor/mcp.json` or `~/.cursor/mcp.json` | <https://cursor.com/docs/context/mcp> |
| Google Antigravity | `~/.gemini/config/mcp_config.json` or `.agents/mcp_config.json` | <https://antigravity.google/docs/ide/mcp/> |

No schema was invented. All four take the same `mcpServers` shape, which is why the README shows
the block once. The documented install line uses PEP 508 direct-reference syntax, because the
`#egg=` form pip 25 will reject was tried first and warned.
