# Gemini Work Order: Independent Code Review of the SNITCH Rust Port

**Prepared:** 2026-08-30
**Subject:** `github.com/3d3dcanada/snitch`, branch `rust-port`, pull request #1
**Mode:** **Review and propose. Do not apply.** Write a report with proposed diffs. Randall takes
the report back to Claude, who applies what he approves.
**Why you:** the code was written by Claude in one session and then reviewed by Claude, which is
worth less than an independent read and is recorded as such at the top of
`docs/REVIEW-RUST-2026-08-30.md`. Codex was asked first and was cut off by an automated filter
before it tested anything. **Nobody has read this code who did not write it.**

---

## Copy/paste invocation

~~~text
Read /home/wess/snitch/docs/HANDOFF-GEMINI-RUST-REVIEW-2026-08-30.md in full, then independently
review the SNITCH Rust port on branch rust-port. This is our own software, about to be published,
and the author reviewed their own work, so what is wanted from you is the read they could not do.
Section 3 is where your value is: read the code rather than only run it, because everything found
so far was found by running things. Section 2 lists what has already been checked so you do not
repeat it. Report and propose diffs only; do not apply changes, do not commit, do not merge, do
not tag, do not publish, and do not touch main or /home/wess/3d3d-site. Save your report at
/home/wess/snitch/docs/REVIEW-GEMINI-RUST-2026-08-30.md with findings by severity, a proposed diff
for each, and a verdict.
~~~

---

## 1 · Orientation

`~/snitch` is a CLI: `snitch` reports what an image is telling people, `no-comment` strips metadata
losslessly, `credit` writes attribution and can sign a C2PA credential. `snitch-mcp` is a local
stdio MCP server over the same core. It was Python until 2026-08-30; the Python is kept verbatim in
`legacy/python/` because it is the specification the port is diffed against.

Read before the code:

- `~/.claude/skills/rust-for-this-machine/SKILL.md`, the binding house style. No async, no web
  framework, no CLI framework, and a dependency needs a reason you would defend out loud.
- `docs/RUST-PORT-2026-08-30.md`, the author's account of the port.
- `docs/REVIEW-RUST-2026-08-30.md`, the author's review of their own work, including its Section 7
  list of gaps it could not close.
- `legacy/python/README.md`.

Get oriented, then form your own view:

```bash
git -C /home/wess/snitch log --oneline main..rust-port
cargo build --release --locked && cargo test && cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

---

## 2 · Already checked. Do not spend your review here.

Everything in this section has been done and is evidenced in `docs/REVIEW-RUST-2026-08-30.md`.
Re-run any of it if you doubt it, but do not treat re-finding it as a finding.

**Six defects found and fixed, each with a test:** a clean file losing its `pixels byte-identical`
proof; a PNG text chunk reaching 3.6 GB; a closed pipe panicking; a phantom dot on an extensionless
file; a lost `FAILED` prefix; and one PNG chunk walker bounding its length arithmetic while its twin
did not.

**Parity, on a 32-fixture matrix** covering progressive, CMYK, grayscale, 1x1, ICC, interlaced,
palette, 16-bit, alpha and animated PNG, files whose extension disagrees with their content, no
extension at all, and every orientation from 2 to 8: `snitch` 64/64 identical including JSON and
exit codes, `no-comment` 32/32 with byte-identical output files, `credit` 32/32 with byte-identical
output files. Coloured output compared separately through a pty.

**Behaviour under bad input:** field values reaching ExifTool as options (they do not); PNG length
arithmetic; FIFOs, device nodes and directories; `--in-place` into a read-only directory; a 191 MB
line into the MCP server; ill-formed JSON-RPC of nine shapes; 24 concurrent strips; 12 processes
racing into one target. All clean.

---

## 3 · Where your value actually is: read the code

Everything found so far was found by **running** the program. Nobody has **read** it. That is the
gap you are here to close, and the items below are where a reader finds things a runner cannot.

### 3.1 The arithmetic

**70 `as` casts** in `src/`, most of them in `src/stamp.rs`, which does layout in `f32` and
`u32` and converts between them constantly.

- Every `f32 as u32` truncates and saturates. Find the ones where an odd or extreme input
  makes the result nonsense: a `--scale` of 0.999 on an 8000px image, an `--opacity` just above 0,
  a logo one pixel tall, a stamp string longer than the image is wide.
- `rounded_rect` computes corner distances in `i32` from `u32` widths. Check the conversion.
- `stamp.rs` uses `saturating_sub` in four places. For each, ask whether saturating is the right
  answer or whether it silently produces a wrong layout instead of an error.
- `png.rs::crc32` and the `const fn crc_table()` are hand-written. Verify the table against a known
  CRC-32 vector rather than trusting that the PNGs happen to round-trip.

### 3.2 The panic surface

Zero `unwrap()`, zero `panic!`, three `expect()`. Judge all three:

- `src/png.rs` twice: `data[i + 4..i + 8].try_into().expect("four bytes")`. The slice is provably
  four bytes given the guard above it. Confirm the guard is where it looks like it is.
- `src/survival.rs:88`: `expect("the built-in survival table is valid JSON")` on an `include_str!`.
  That is a build-time invariant, but nothing enforces it at build time. Should it?

Then find what the count misses: slice indexing, integer division, and array access that panics
without any of those words appearing.

### 3.3 The one `unsafe`

`src/cli.rs`, restoring the default SIGPIPE disposition with `libc::signal`. Read the safety
comment and decide whether it is true. It runs before any thread is spawned; verify that claim
against the actual `main` functions in all four binaries.

### 3.4 The C2PA logic against the specification

`src/c2pa.rs` decides three separate facts: presence, asset-binding integrity, signer trust. It is
the part of this tool that could most easily mislead a person about a real image.

- `ALTERED_CODES` lists three status codes. Check that against the C2PA specification's validation
  status codes. Is the list complete? Is anything in it wrong?
- `digital_source` matches on substrings of `digitalSourceType`. Is substring matching safe here, or
  can a legitimate URL contain `trainedAlgorithmicMedia` as a substring of something else?
- `refine_with_metadata` upgrades `Unavailable` to `DetectedUnverified` on
  `meta["JUMBF:JUMDLabel"] == "c2pa"`. Is that the right signal? Are there containers it misses?
- `read_report` treats `"No claim found"` in the tool's output as `Absent`. That is string matching
  against another program's error text. What happens when c2patool changes its wording?

### 3.5 The MCP server against the specification

`src/mcp.rs` is hand-written JSON-RPC 2.0. Check it against the current MCP specification, not
against what it does.

- The `inputSchema` for each tool: are the types and `required` lists right? Does a model reading
  them get an accurate picture of what the tool accepts?
- `initialize` is not required before `tools/call`. Defect or not?
- Errors come back as a tool result with `isError`, not a JSON-RPC error. Confirm that is what the
  specification wants for a tool that declined rather than crashed.
- Two known nits are listed in the review's Section 6 and were deliberately left. Second-guess that.

### 3.6 The tests, as code

Read `tests/` as an artefact in its own right.

- Which tests would still pass if the behaviour they name were broken? Name them.
- `tests/common/mod.rs` builds fixtures. Do they actually contain what the tests assume?
- Several tests begin `if !have("exiftool") { return; }`, which **passes silently** rather than
  skipping visibly. On a machine without ExifTool a green run means very little. Propose better.
- The parity harness lives in a scratch directory and is not in the repository. It is the single
  most valuable test asset here and it will be lost. Propose where it should live.

### 3.7 Rust idiom and the house style

- Is anything here fighting the borrow checker in a way that suggests the wrong shape?
- `src/inspect.rs` holds ordered `Vec<(String, Value)>` and serialises them as maps. Reasonable, or
  a `Vec` pretending to be an `IndexMap`?
- `src/cli.rs::Args` is a hand-rolled parser. Read it for cases it gets wrong: `--flag=value` on a
  flag, a repeated option, `-` alone, an option consuming the next option as its value.
- Seven direct dependencies. Is each defensible? The refusals of `c2pa` (280 crates) and `rmcp`
  (59 crates plus tokio) are argued in `Cargo.toml`. Are the arguments sound, or is that a rule
  being applied past the point where it helps?
- Do the comments carry the measurement that forced the decision, as the house style requires, or
  are they decoration?

### 3.8 The claims a user reads

This tool's whole value is that it does not overstate. Read every user-facing string in `src/` and
`README.md` and find any that claims more than the code delivers.

Specifically verify these hold: a missing C2PA credential is never evidence an image is real; a
self-signed certificate is never presented as verified identity; nothing claims to remove an
in-pixel watermark; `ai_source` is always present when `ai` is; `src/text.rs` never strips ZWJ or
ZWNJ.

---

## 4 · Rules

- **Report and propose. Do not apply, commit, merge, tag or publish.**
- Do not touch `main`, and do not touch `/home/wess/3d3d-site`, which another agent owns.
- Do not add a dependency in a proposal without measuring its tree with `cargo tree`.
- Do not propose async, a web framework or a CLI framework without the measured cost, and expect it
  to be refused.
- `legacy/python/` is the specification. If the Rust and the Python disagree, the Python is presumed
  right until you argue otherwise, and any proposal that changes byte-identical output must say so
  loudly.
- No em dashes in anything you write.
- **Do not report anything as checked that you did not run**, and name the environment you ran it
  in. That is the exact mistake that let CI stay red for four days here.

---

## 5 · Deliverable

`/home/wess/snitch/docs/REVIEW-GEMINI-RUST-2026-08-30.md`, containing:

1. **Verdict** on one line at the top: ship, ship with the listed fixes, or do not ship.
2. The state you observed and the environment you ran in.
3. **Findings by severity.** P0 correctness, data loss, a false claim reaching a user, or a crash on
   input a user did not write. P1 portability, packaging, or a gap a real user will hit. P2 style,
   house rule, taste.
4. For each finding: **file and line, the input or condition that triggers it, observed behaviour,
   expected behaviour, and a proposed diff.** A finding without a reproduction is a guess and should
   say so.
5. **What you read and found nothing wrong with.** A review that only lists problems does not tell
   Randall which parts were actually examined.
6. Anything in Section 3 you could not get to.
7. Your view on the seven open gaps in `docs/REVIEW-RUST-2026-08-30.md` Section 7.
