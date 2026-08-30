# Codex Work Order: Adversarial Engineering Review of the SNITCH Rust Port

**Prepared:** 2026-08-30
**Subject:** `github.com/3d3dcanada/snitch`, branch `rust-port`, pull request #1
**Mode:** Review and report. Do not merge, tag, publish, deploy, or push to `main`. You may create
a branch and commit fixes only if Section 9 tells you to, and only there.
**Authority:** Ken, 2026-08-30: *"give me a prompt for Codex to go over this and ensure that the
Rust is built properly."* The port was written by Claude in one session. Nobody else has read it.

---

## Copy/paste invocation

~~~text
Read /home/wess/snitch/docs/HANDOFF-CODEX-RUST-REVIEW-2026-08-30.md in full, then adversarially
review the SNITCH Rust port on branch rust-port. Your job is to find what is wrong with it, not to
agree with it. Every claim in Section 3 is a claim the author made about their own work: reproduce
each one independently and report the ones that do not hold. Build it, test it, fuzz the parsers,
read the unsafe block, attack the MCP server, and check the whole thing against
~/.claude/skills/rust-for-this-machine. Section 4 lists deviations the author already knows about,
so do not spend the review re-finding those; judge whether each was the right call and find the
ones they missed. Do not merge, tag, publish or touch main. Save a dated report at
/home/wess/snitch/docs/REVIEW-CODEX-RUST-2026-08-30.md with exact commands, exact output, findings
by severity, and a verdict.
~~~

---

## 1 · What this is

`~/snitch` is a standalone CLI: `snitch` reads what an image is telling people, `no-comment` strips
it losslessly, `credit` writes attribution and optionally signs a C2PA credential. `snitch-mcp` is
a local stdio MCP server over the same core.

It was Python. On 2026-08-30 it was ported to Rust in one session, and the Python was moved to
`legacy/python/` verbatim rather than deleted, because it is now the specification the port is
diffed against.

**Read these before the code:**

- `~/.claude/skills/rust-for-this-machine/SKILL.md` is the binding house style. The port claims to
  follow it. Judge that claim.
- `docs/RUST-PORT-2026-08-30.md` is the author's own account of the port, including the two
  dependencies refused and the bugs found on the way.
- `docs/AUDIT-CLAUDE-SNITCH-2026-08-29.md` is the audit of the Python that preceded it. Its
  findings about truthfulness carry into the Rust, and Section 8 of it lists gaps that are still
  gaps.
- `legacy/python/README.md` explains why the Python is still here.

---

## 2 · Required state check, before anything else

```bash
git -C /home/wess/snitch status --short --branch
git -C /home/wess/snitch log --oneline main..rust-port
gh -R 3d3dcanada/snitch pr view 1 --json state,mergeable,headRefOid
gh -R 3d3dcanada/snitch run list --branch rust-port --limit 5
rustc --version && cargo --version
which exiftool c2patool openssl && exiftool -ver && c2patool --version
```

Record what you actually see. If the branch has moved since this was written, review what is
there, and say so.

**A thing worth knowing up front:** CI on `main` had been failing on every job since the repository
was published on 2026-08-25. The audit of 2026-08-29 declared the Python's gates green because it
ran them locally on one Python on one OS and never looked at `gh run list`. The cause was one test
asserting an exit code the code cannot produce. That is the class of miss this review exists to
catch, so **check the CI, do not take a local green as evidence.**

---

## 3 · The claims. Reproduce every one independently.

These are the author's claims about their own work. Treat each as unproven. For each, either
confirm it with your own command and output, or report exactly how it fails.

### 3.1 Parity with the Python

The strongest claim, and the one the whole port rests on: against the same fixtures, both
implementations produce byte-identical terminal output, byte-identical JSON, identical exit codes,
and byte-identical output files.

| Claimed | |
| --- | --- |
| `snitch` | 15 / 15 invocations identical, including the full C2PA manifest |
| `no-comment` | 8 / 8 identical, including the stripped output files themselves |
| `credit` | 15 / 18 identical; the 3 differ only by argparse's usage block |

**Build your own harness. Do not reuse the author's.** Install the legacy Python
(`cd legacy/python && pip install -e '.[dev]'`), build the Rust release binaries, and diff them
across a fixture matrix you design. Cover at minimum: a JPEG with EXIF, GPS, IPTC, XMP and Unicode;
a PNG with `tEXt`, `zTXt` and `iTXt`; a C2PA-signed file; a tampered signed file; a file with an
orientation tag; a CMYK JPEG; a JPEG with an ICC profile; a progressive JPEG; an interlaced PNG; a
palette PNG; a 16-bit PNG; an APNG; a PNG with an alpha channel; a grayscale image; a 1x1 image; a
file with no extension; a file with the wrong extension for its content.

**The orientation path is the one most likely to be wrong and least likely to be covered.** The
Rust hand-writes a 32-byte TIFF block in `src/exif.rs::orientation_payload` to match what Pillow's
`Image.Exif().tobytes()` produced. Verify it byte for byte for every orientation value 2 through 8,
and verify that a stripped file still displays rotated in something that is not this tool.

### 3.2 The measurements

| Claimed | Where |
| --- | --- |
| `snitch-mcp` idle RSS: 2.4 MB Rust, 66.8 MB Python | README, PR |
| All four binaries: 4.3 MB | README |
| 47 crates in the tree, 7 direct, zero async | `Cargo.toml`, README |
| `c2pa` crate is 280 crates | `Cargo.toml` comment |
| `rmcp` crate is 59 crates and brings tokio | `Cargo.toml` comment |

Re-measure all of these. `cargo tree`, `/proc/<pid>/status`, `ls -la`. If a number is wrong the
comment justifying a decision is wrong, and the decision needs re-examining.

### 3.3 The gates

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo test --doc
cargo build --release --locked
cd legacy/python && ruff check snitch tests && mypy snitch && pytest -q
```

Then run the Python suite **without c2patool on PATH and with a clean HOME**, which is what CI
sees, because that difference is what kept CI red for four days.

---

## 4 · Deviations the author already knows about

Do not spend the review re-finding these. Judge whether each was the right call, and say so.

1. **`credit` prints its own usage block, not argparse's.** Three parity cases differ for this
   reason alone; the error line beneath is byte-identical and the exit code matches. Consequence of
   the no-CLI-framework rule.
2. **The stamp finds a font at runtime instead of embedding one.** Pillow shipped a face; embedding
   a TTF would add most of a megabyte to every binary. `src/stamp.rs::FONT_CANDIDATES` is the search
   list and the error names `--font` when nothing is found. **On a machine with no system font,
   `credit --stamp` fails where the Python worked. Judge whether that trade is acceptable, and
   whether the candidate list is adequate on macOS and Windows.**
3. **The stamp carries metadata across the re-encode with `exiftool -TagsFromFile -all:all`**, where
   Pillow carried only `exif` and `icc_profile`. That is strictly more, and it makes `--stamp
   --keep-gps` work as documented. It also makes `credit --stamp` require ExifTool where the Python
   did not. Judge it.
4. **Stamped pixels are not byte-identical to the Python's**, because `ab_glyph` is not FreeType and
   `image`'s Lanczos is not Pillow's. Only the metadata and the layout are compared.
5. **A closed pipe now exits 141 (SIGPIPE), where the Python exits 0.** This was a panic until it
   was fixed. 141 is the conventional Unix answer. Judge whether 0 would be better.
6. **`libc` is a seventh direct dependency, unix only, for two lines of unsafe** that restore the
   default SIGPIPE disposition. It is the only `unsafe` in the codebase. Read it.
7. **`serde_json` carries `preserve_order`**, costing three transitive crates, so JSON reports diff
   cleanly across an upload round trip.
8. **`rustfmt` is enforced here and is not enforced in `ora-runtime`.** A deliberate divergence from
   the house codebase.
9. **The survival table is `include_str!`'d JSON with a `SNITCH_SURVIVAL` environment override.**
   The override reads an arbitrary path. Judge the risk.

---

## 5 · Where to actually attack it

This is the part of the review that matters. The tests pass; find what the tests do not cover.

### 5.1 The parsers, which take untrusted bytes

`src/png.rs` and `src/jpeg.rs` walk attacker-controlled files by hand.

- **Fuzz them.** `cargo-fuzz` or `afl.rs` against `png::read_text`, `png::strip`, `jpeg::strip` and
  `exif::orientation_payload`. Malformed lengths, lengths that overflow, chunks claiming more bytes
  than the file has, zero-length chunks, a `zTXt` whose zlib stream is a decompression bomb, an
  `iTXt` with a compression flag set and no data, nested NULs in keywords, 4 GB length fields.
- **`png.rs` has `let total = length + 12;` in `strip`.** `length` comes from the file. Prove
  whether that can overflow on a 32-bit target, and whether the `i + total > data.len()` check that
  follows is reached before any indexing.
- **`read_text` deliberately swallows malformed structure and returns what it read.** Prove it
  cannot loop forever on a chunk with `length == 0` and a type that is not `IEND`.
- **`jpeg.rs::strip` scans for `FF D9` with a windows() search over the remaining bytes.** Check the
  cost on a large file and whether a crafted file can make it quadratic.
- **The zlib inflate in `png.rs::inflate` has no output size limit.** A small `zTXt` chunk can
  decompress to gigabytes. Establish whether that is reachable and what it costs.

### 5.2 The MCP server, which takes untrusted lines

`src/mcp.rs::serve` is a hand-rolled JSON-RPC loop.

- **`BufRead::lines()` has no length limit.** One line with no newline exhausts memory. Establish
  whether that matters for a local stdio server whose peer is the host that launched it, and say so
  either way rather than assuming.
- Send: an id that is an object, a float, null, a huge integer. A `tools/call` with `arguments` as
  an array. A method name of 10 MB. Duplicate ids. A response frame instead of a request. Batched
  requests, which the spec once allowed. Verify none of it panics and none of it writes anything to
  stdout that is not a frame.
- **Prove the stdout purity claim harder than the existing test does.** Make ExifTool fail loudly,
  make c2patool fail loudly, and confirm neither reaches the wire.
- Check `initialize` is not required before `tools/call` and decide whether that is a defect.

### 5.3 The filesystem paths, which are where real damage lives

- `strip::temporary_sibling` loops `0..4096` with `create_new(true)`. Race-check it.
- `strip::strip_atomic` renames across the same directory. Prove nothing partial can appear at the
  target, including when the process is killed mid-write, and when the target directory is
  read-only, full, or on a different filesystem from the source.
- `strip::copy_permissions` uses `fs::metadata`, which follows symlinks. Establish whether that
  matters here.
- `mcp::source_path` refuses symlinks. Check the parent directories: a path through a symlinked
  directory is not refused. Decide whether it should be.
- `credit --in-place` and `no-comment --in-place` replace the user's file. Prove the failure paths
  leave the original intact.
- Try every command against: a FIFO, a device node, a file the user cannot read, a file that
  disappears between the check and the open, a filename that is invalid UTF-8, a path longer than
  PATH_MAX, a directory named `photo.jpg`.

### 5.4 The subprocess boundary

Every ExifTool and c2patool call passes user-controlled values.

- `exif::Credit::args` builds `-XMP-dc:Creator={value}` strings. A creator name beginning with `-`,
  containing a newline, or containing `=`. ExifTool has `-@` argfile syntax and `-execute`; prove a
  crafted field value cannot reach either.
- `sign::subject_value` escapes `/` and `\` for an OpenSSL subject. Prove that is sufficient.
- Confirm no call inherits stdout, and that `C2PA_PRIVATE_KEY` and `C2PA_SIGN_CERT` are cleared on
  the signing path as `sign.rs` claims.

### 5.5 The truth claims, which are the product

The audit of 2026-08-29 exists because this tool's value is that it does not lie. Verify the Rust
kept every one:

- A missing C2PA credential is never reported as evidence an image is real.
- `detected-unverified` still exists and never collapses into `absent`.
- A self-signed certificate is never presented as verified identity.
- `asset_altered` and `identity_untrusted` are never merged.
- Nothing anywhere claims to remove an in-pixel watermark.
- `ai_source` is always present when `ai` is, so a PNG text chunk can never read as a signed claim.
- The platform table still marks every unverified row unverified.
- `src/text.rs` never strips ZWJ or ZWNJ. Test it with real emoji sequences, Persian, Kurdish,
  Devanagari and Bengali beyond the cases already in `tests/text.rs`.

### 5.6 The house style

Judge against `~/.claude/skills/rust-for-this-machine`, not against generic Rust taste.

- Zero async: verify with `cargo tree`, not by reading.
- Seven direct dependencies: is each one defensible out loud? The refusals of `c2pa` and `rmcp` are
  argued in `Cargo.toml` comments. Are the arguments sound?
- Do the comments carry the measurement that forced the decision, or are they decoration?
- Does any error message fail to name the fix?
- Is there a silent fallback anywhere? The house rule is that degradation is reported, never
  swallowed. `stamp::carry_metadata` prints to stderr and continues; judge it.

---

## 6 · What was never tested at all

State these as gaps in your report rather than assuming they work.

1. **macOS and Windows**, beyond what CI does. No human has run these binaries on either.
2. **A real MCP host.** The server has been driven by the official Python client over a pipe. It has
   never been configured into Claude Code, Claude Desktop, Cursor or Antigravity and used. The
   config snippets in the README were each checked against that host's current documentation, but
   none was executed.
3. **The release workflow.** It has never run. The packaging step was dry-run by hand on Linux only.
   The Windows `7z` branch and the ARM runners are unexercised.
4. **Large files.** Nothing above a few hundred kilobytes has been through it.
5. **Platform round-trips.** Every row of the survival table is still unverified inference, dated
   `2026-08-25`, and honestly labelled as such.
6. **Concurrency.** Two processes stripping the same file into the same target.
7. **`cargo install --git`**, which the README tells people to run, from a clean machine.

---

## 7 · Hard rules for this review

- **Do not merge, tag, publish, or push to `main`.** The release is Ken's call.
- **Do not publish to crates.io or PyPI.** Both are outward-facing and effectively irreversible.
- **Do not delete `legacy/python/`.** It is the specification. If you find the Rust and the Python
  disagree, the Python is presumed right until argued otherwise.
- **Do not add a dependency** without measuring its tree and arguing for it the way `Cargo.toml`
  argues for the seven that are there.
- **Do not introduce async, a web framework, or a CLI framework.** If you believe one is warranted,
  bring the measured cost and let Ken decide.
- **Do not touch `/home/wess/3d3d-site`.** Another agent owns it. The web-side audit findings
  (LIVE-1, LIVE-2, P1-2, the `/snitch` footer) are theirs.
- **No em dashes** in anything you write.
- **Do not report a gate as clean that you did not run**, and name the environment you ran it in.
  That is the exact mistake that let CI stay red for four days.

---

## 8 · Deliverable

Save `/home/wess/snitch/docs/REVIEW-CODEX-RUST-2026-08-30.md` containing:

1. **Verdict:** ship, ship with the listed fixes, or do not ship. One line, at the top.
2. The state you observed, and the environment every command ran in.
3. **A claim-by-claim table** for Section 3: confirmed, refuted, or unverified, with the command and
   the output that decided it.
4. Findings by severity. **P0** correctness, data loss, a false claim reaching a user, or a
   security defect. **P1** portability, packaging, a gap a real user will hit. **P2** style, house
   rule, taste.
5. For each finding: the file and line, the input that triggers it, the observed behaviour, and the
   behaviour you expected. A finding without a reproduction is a guess and should be labelled one.
6. **Every gap you could not close**, and why.
7. **Your judgement on each of the nine deviations in Section 4.** Right call or wrong call, and
   what you would do instead.
8. Exactly one recommended next work order.

---

## 9 · If you are asked to fix rather than only report

Only if Ken says so explicitly.

- Branch from `rust-port`, never from `main`.
- One finding per commit, with the reproduction in the message.
- Every fix carries a test that fails before it and passes after.
- Re-run the full parity harness after every change. If a fix breaks byte-identical output with the
  Python, that is a finding in itself and needs Ken, because the Python is the specification.
- All gates green in both environments before you hand it back: with c2patool, and without.
