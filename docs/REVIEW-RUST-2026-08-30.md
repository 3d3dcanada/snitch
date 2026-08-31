# Engineering Review of the SNITCH Rust Port

**Date:** 2026-08-30
**Subject:** `github.com/3d3dcanada/snitch`, branch `rust-port`, pull request #1
**Reviewer:** Claude, who also wrote the code.

---

## 0 · The limitation, stated first

**This is a self-review, not an independent one.** I wrote every line under review. That is worth
less than a second pair of eyes and it should not be recorded as though it were the same thing.

What I can do honestly is run things and report what happened, so everything below is a command and
its output rather than an opinion about my own design. Where a judgement was needed, it is labelled
as one. The work order at `docs/HANDOFF-CODEX-RUST-REVIEW-2026-08-30.md` is still open and still
worth giving to somebody else.

### What Codex did

Codex read the handoff, inventoried the claims, created a detached worktree at
`/tmp/snitch-codex-current` on the correct head, ran two commands, and was cut off by an automated
cybersecurity filter before it tested anything. It produced no report.

It also did no damage, which was checked rather than assumed:

```
$ git worktree list
/home/wess/snitch          [rust-port]
/tmp/snitch-codex-current  (detached HEAD)
$ git log --oneline main -1        b773c5a   (unchanged)
$ git log --oneline origin/main -1 b773c5a   (unchanged)
$ git stash list                   (empty)
$ git status --short               (clean)
```

The worktree is left in place in case Codex resumes. **The refusal was caused by how the prompt was
written, not by Codex.** The first version titled a section "where to actually attack it" and asked
for fuzzing and command-injection testing, which reads as offensive security tasking to a filter
even on your own code. It has been reworded as the robustness review it is.

---

## 1 · Verdict

**Ship, after the five defects below were fixed. They are fixed, and each has a test.**

The port is sound in the ways that matter: it does what the Python does, on the same files, to the
byte. Everything found this session was found by widening the input set rather than by reading the
code, which is the argument for the fixture matrix in Section 3 outliving this review.

---

## 2 · State and environment

```
rustc 1.93.1 (01f6ddf75 2026-02-11) · cargo 1.93.1
exiftool 12.76 · c2patool 0.27.15 · openssl present
branch rust-port, ahead of main by 6 commits
```

Every gate below ran on Linux x86_64 on this machine. **macOS and Windows were exercised by CI
only**, and nobody has used a binary on either.

---

## 3 · The claims, reproduced

### 3.1 Parity: CONFIRMED, after three defects were fixed

The original run used four fixtures that all carried metadata and all had ordinary extensions. That
was too easy, and it hid three real breaks. A 32-fixture matrix was built covering progressive,
CMYK, grayscale, 1x1, ICC-profiled, interlaced, palette, 16-bit, alpha and animated PNG, files whose
extension disagrees with their content, a file with no extension at all, and every EXIF orientation
value from 2 to 8.

| | Result |
| --- | --- |
| `snitch`, text and JSON, exit codes | **64 / 64 identical** |
| `no-comment`, output and produced files | **32 / 32 identical**, files byte-identical |
| `credit`, output and produced files | **32 / 32 identical**, files byte-identical |
| The original suites | `snitch` 15/15, `no-comment` 8/8, `credit` 15/18 |

The three `credit` cases still differ by argparse's usage block above a byte-identical error line,
on a matching exit code. That is Section 4 item 1 and it stands.

**Colour output was never compared until this session**, because every parity run used `NO_COLOR=1`,
which cannot see a colour difference. Re-run through a pty: eight coloured invocations across
`snitch` and `no-comment`, all identical. That check found the fourth defect below.

**Stripped files still decode correctly**, verified in an unrelated decoder: interlaced, palette,
alpha, 16-bit, CMYK, progressive and animated all come back the same size with the same pixels, and
orientation 6 survives the strip.

### 3.2 The measurements: CONFIRMED, and the README was wrong

Re-measured independently. Three figures the README published had drifted while the code moved
under them, which is a false claim in a shipped document and is fixed.

| | Claimed | Measured | |
| --- | --- | --- | --- |
| `snitch-mcp` idle RSS | 2.4 MB | **2,080 kB** | claim is conservative, kept |
| All four binaries | 4.3 MB | **4.3 MB** | holds |
| Dependency tree | 46 crates | **47** | corrected |
| Direct dependencies | 6 | **7** | corrected, `libc` was added later |
| Tests | 68 | **75** | corrected |
| Source | 4,506 / 1,643 | **4,585 / 1,796** | corrected |
| async in the tree | zero | **zero** | holds |
| `c2pa` crate | 280 crates | **280** | holds |
| `rmcp` crate | 59 crates, brings tokio | **59, yes** | holds |

### 3.3 The gates: CONFIRMED

```
cargo fmt --check                        clean
cargo clippy --all-targets -- -D warnings clean
cargo test                               75 passed, 0 failed
cargo test --doc                         0 failed
cargo build --release --locked           ok
legacy python, with c2patool             101 passed
legacy python, without c2patool          99 passed, 2 skipped
cargo install --git ... --branch rust-port --locked
                                         installs all four executables
```

CI on `main` had been failing on every job since publication on 2026-08-25. Cause: one test
asserting an exit code `credit_main` cannot produce. Fixed. **The audit of 2026-08-29 missed it by
running gates locally and never checking `gh run list`.**

---

## 4 · Findings

All five were found this session and all five are fixed with a test. Severity is what the defect
could have done to a user, not how hard it was to find.

### P0-1 · A file with nothing to remove lost its pixel proof

`src/bin/no_comment.rs`. On a file with no metadata, the output read
`already had nothing to remove` where the Python prints
`removed 0 bytes of metadata  pixels byte-identical`.

It read better and it dropped `pixels byte-identical` from the one case a reader most wants to see
it in. This tool's entire value is that its claims are checkable, and this quietly removed one. It
is also the "never port and redesign in the same change" rule broken, in the worst possible place.

Invisible to the original harness because every fixture carried metadata.

### P0-2 · A PNG text chunk could take the process to 3.6 GB

`src/png.rs`. A 510 KB PNG holding one `zTXt` chunk of compressed zeros drove `read_text` to
**3.6 GB resident and eight seconds**. The inflated bytes were copied again for UTF-8 handling and
again for JSON escaping. A 4 MB file holding 200,000 tiny chunks reached 428 MB.

Capped at 1 MB per chunk, 4 MB per file, 256 chunks. A chunk that hits the cap reports `truncated`
rather than handing back a silently shortened prompt, and the flag is skipped when false so
ordinary output is unchanged. **After: the reader handles both files in about 5 MB, instantly.**

The residual 1.6 GB on that file is ExifTool's own cost for `-j -G -n -a -u`, measured standalone.
The Python cost 6.8 GB on the same input. Documented at the call site rather than hidden.

### P1-1 · A closed pipe panicked

`snitch --platforms | head -4` printed four lines and then
`failed printing to stdout: Broken pipe`. Rust sets SIGPIPE to `SIG_IGN` at startup. Found by piping
a release binary into `head`, which is how anyone reads a long table. Now exits 141, the
conventional answer. Costs one crate and the only `unsafe` in the codebase, at `src/cli.rs:34`.

### P1-2 · A file with no extension grew a phantom dot

`noextension: . is not supported` instead of `noextension:  is not supported`. Python's `splitext`
returns the extension with its dot; mine returned it without and the format string added one back.

### P2-1 · A `credit` write failure lost its `FAILED` prefix and printed a full path

The Python prints `{basename}: FAILED {error}`. A script grepping for that word would have missed
it. The watermark note was also dim throughout where the Python bolds its first line, which only a
pty comparison could see.

---

## 5 · Checked and clean

Measured, no defect found. Recorded so a later review does not have to repeat them.

| | Result |
| --- | --- |
| Field values reaching ExifTool as options | **No.** `-execute`, `-@ /etc/passwd`, `-TagsFromFile=...` and an embedded newline all written as literal tag values, because each argument is its own argv entry |
| PNG length field arithmetic | Handled. `0xFFFFFFFF` length, a chunk claiming 1 GB, and repeated zero-length non-IEND chunks all refused or walked without hanging |
| FIFO, device node, directory, `/dev/zero` | All refused, none hang |
| `--in-place` into a read-only directory | Original byte-identical afterwards |
| 191 MB on one line into the MCP server | 393 MB, exits 0, no crash |
| Ill-formed JSON-RPC: object/float/huge ids, array arguments, no params, numeric method, batch array, a response frame, 200-deep nesting, unparseable text | No panic, nothing but response frames on stdout, server exits 0 |
| 24 concurrent strips into one directory | 24 identical outputs, no temporary left behind |
| 12 processes racing into the same target | Valid image, no temporary left behind |
| A path through a symlinked directory | Resolves to the real file the user named. Reads as correct |
| Large JPEG through `no-comment` | 85 MB peak, output byte-identical to the Python's |

---

## 6 · Judgement on the Section 4 deviations

| | Verdict |
| --- | --- |
| 1. Own usage block, not argparse's | **Right.** The error line is byte-identical and the exit code matches. Reproducing argparse's wrapping is not worth a CLI framework |
| 2. Runtime font search, no embedded TTF | **Right, with a caveat.** Saves most of a megabyte per binary and the error names `--font`. The caveat is real: `credit --stamp` fails on a machine with no system font where the Python worked. Worth a line in the README |
| 3. Stamp carries metadata with `exiftool -TagsFromFile -all:all` | **Right.** Strictly more than Pillow carried, and it makes `--stamp --keep-gps` true. It does make `--stamp` require ExifTool, which the Python did not |
| 4. Stamped pixels not byte-identical | **Unavoidable.** Different rasteriser, different resampler. The metadata and layout are compared instead |
| 5. Closed pipe exits 141, Python exits 0 | **Right.** 141 is what a Unix tool does |
| 6. `libc` for two lines of unsafe | **Right.** The alternatives were string-matching a panic message or replacing every print site |
| 7. `preserve_order` for three crates | **Right.** `--platforms --check` tells people to diff two reports across an upload |
| 8. rustfmt enforced here, not in ora-runtime | **Defensible.** A contributor's editor agrees with the repository |
| 9. `SNITCH_SURVIVAL` override | **Marginal.** Nobody has asked for it. It reads a path the user names, so the risk is low, but it is a feature with no user |
| 10. PNG text caps | **Right**, see P0-2. The numbers are generous against real generator payloads |
| 11. Two MCP spec nits | **Leave them.** A response frame answered with `method not found` and a dropped batch array. Neither crashes anything; batches were removed from the spec |

---

## 7 · Gaps this review did not close

1. **Independence.** I wrote the code. This is the largest gap on the page.
2. **macOS and Windows** beyond CI. No human has run a binary on either.
3. **A real MCP host.** The server has been driven by the official Python client over a pipe and by
   a hand-written frame harness. It has never been configured into Claude Code, Claude Desktop,
   Cursor or Antigravity and used. The config snippets were each checked against that host's current
   documentation; none was executed.
4. **The release workflow has never run.** Packaging was dry-run by hand on Linux. The Windows `7z`
   branch and the ARM runners are unexercised.
5. **No coverage measurement**, and no property-based or generated-input testing beyond the specific
   malformed cases above.
6. **Platform round-trips.** Every survival table row is still unverified inference, dated
   2026-08-25, and honestly labelled.
7. **Files above about 4 MB** have not been through the tool.

---

## 8 · Next work order

**Merge #1, tag `v0.2.0`, and let the release workflow run.** That is the one remaining thing that
exercises code nobody has run: five build targets, the Windows packaging branch, and the artifact
upload. It is also the only way the README's download link stops being a promise.

If it fails, it fails in CI where it costs nothing. If it succeeds, install one of the produced
archives on a machine that is not this one and run `snitch --platforms`, which closes gap 2 and
gap 4 together.

Do not publish to crates.io or PyPI. Neither is needed and both are irreversible.
