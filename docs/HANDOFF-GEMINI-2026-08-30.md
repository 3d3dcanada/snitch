# Gemini Work Order: Review the SNITCH Rust Port, then Build its Developer Hub

**Prepared:** 2026-08-30
**Two parts, in order.** Part A is a code review with no changes applied. Part B is a build.
**Do not start Part B until Part A's report is written**, because what Part A finds may change what
Part B should say about the software.

---

## Copy/paste invocation

~~~text
Read /home/wess/snitch/docs/HANDOFF-GEMINI-2026-08-30.md in full, then do Part A and Part B in that
order.

Part A: independently review the SNITCH Rust port on branch rust-port, following
/home/wess/snitch/docs/HANDOFF-GEMINI-RUST-REVIEW-2026-08-30.md. The author reviewed their own
work, so what is wanted is the read they could not do: read the code rather than only run it,
because everything found so far was found by running things. Report and propose diffs only. Do not
apply, commit, merge, tag or publish anything in /home/wess/snitch. Save the report at
/home/wess/snitch/docs/REVIEW-GEMINI-RUST-2026-08-30.md.

Part B: build the SNITCH developer hub on the /snitch page in /home/wess/3d3d-site, following Part
B of the handoff. That repository has deployed but uncommitted work from a previous session in its
working tree: do not disturb it, and commit only files you create or change yourself. Obey
3d3d-site's AGENTS.md and CLAUDE.md, verify at 390px and desktop in Ken's Brave through
claude-in-chrome and never Playwright, and do not deploy. Ken reviews before it ships.
~~~

---

# Part A · Review the Rust

The work order is `/home/wess/snitch/docs/HANDOFF-GEMINI-RUST-REVIEW-2026-08-30.md`. Read it in
full. In short:

- The code was written by Claude in one session and then reviewed by Claude, which is worth less
  than an independent read and is said so at the top of `docs/REVIEW-RUST-2026-08-30.md`.
- **Six defects have already been found and fixed.** Section 2 of that work order lists them and the
  parity evidence. Do not spend the review re-finding them.
- **Everything found so far was found by running the program. Nobody has read it.** Section 3 is the
  read: 70 `as` casts and the layout arithmetic in `stamp.rs`, three `expect`s, the one `unsafe`
  block and whether its safety comment is true, the C2PA logic against the published specification,
  the MCP schemas against the current spec, and the tests as an artefact.
- **Report and propose. Do not apply.** Diffs go back to Ken, who decides.

Deliverable: `/home/wess/snitch/docs/REVIEW-GEMINI-RUST-2026-08-30.md`.

---

# Part B · Build the developer hub

## B1 · What it is and why it does not exist yet

`3d3d.ca/snitch` is a free browser tool: drop a photo in, see what it is telling people, take it
out, put your name on it. It is good and it is finished. What it does not do is tell anyone that
the same three tools exist as a command line program and as an MCP server for an AI assistant.

An audit on 2026-08-29 found no developer hub anywhere in the site, and found that building one
then would have shipped two false claims: `pip install snitch-tools` returns HTTP 404, and the
repository had no releases, so a download button would have pointed at nothing. **Both are being
fixed on the `rust-port` branch.** The tool is Rust now, the install line is real, and a release
workflow exists that builds binaries for five targets.

## B2 · The sequencing that keeps it honest

**A release does not exist yet.** Pull request #1 is open and unmerged; there are no tags.

So the hub must degrade honestly and by construction:

- **When no release exists:** show the source install and no download button. Nothing false.
- **When `v0.2.0` is tagged and the release workflow has run:** the download buttons appear with
  real files behind them.

Build it so that transition needs no code change. `app/ui/FormaDownload.tsx` in that repository
already does exactly this job for FORMA, including guessing the platform from the user agent,
printing honest file sizes, and offering every build rather than only the guess. **Read it first and
follow its shape.** Do not invent a second pattern.

If you cannot make it degrade cleanly, ship the source install only and say in your report that the
download block is waiting on the release. **Do not build a button that has nothing behind it.**

## B3 · What goes on the page

Four things a person who wants the command line version actually needs. Nothing else.

1. **What it is, in a sentence or two.** The same three tools, on your own machine, on whole folders
   instead of one file at a time.
2. **Install.** The source install now, the binary download when a release exists. ExifTool is
   required. c2patool is required for signing and full C2PA validation. `credit --stamp` needs a
   system font.
3. **The MCP server.** What the five tools do, and a copyable config block. Every host uses the same
   shape, so show the block once and list where it goes: Claude Code `.mcp.json`, Claude Desktop
   `claude_desktop_config.json`, Cursor `.cursor/mcp.json`, Antigravity
   `~/.gemini/config/mcp_config.json`. **These four were each verified against that host's own
   current documentation on 2026-08-30; the links are in `snitch/README.md`. Do not invent a fifth
   host and do not change a schema.**
4. **The repository.** A link, the licence, and that the Python it was ported from is kept in
   `legacy/python/` as the specification.

**What must NOT go on it:** a `pip install snitch-tools` line, which 404s. A star count or any
number that will go stale. A download button with no release behind it. Anything that implies the
command line tool uploads a file, because it does not, and neither does the web one.

## B4 · Where it goes

`/snitch` is a one-panel app. The panel is the page and the visitor came to use it.

**The hub goes below the tool, not beside it and not above it.** A person who came to strip a photo
must not have to scroll past a developer section to reach the drop zone. Read
`app/snitch/Panel.tsx` and `app/routes/snitch.tsx` before deciding where the seam is.

Ken rejected a stacked page of separate tools once already, which is why the panel exists in the
shape it does. Do not undo that.

## B5 · The rules that bind this work

`/home/wess/3d3d-site/AGENTS.md` and `CLAUDE.md` are binding. Read both in full. The ones that will
bite you:

- **Never a white card with a hard shadow. Never a dark bordered card grid on a moody ground.**
  Cards are light, warm and outcome-led. Depth comes from hairlines and glass, never box-shadow.
- **No gradient text, no gradient logos, no rainbow card grid.** Solid neon accents, teal `#04D9C4`
  leads. Never recolour the wave logo.
- **Title Case labels. Geist and Geist Mono, self-hosted, no CDN.** Choices are premium rows, never
  pills.
- **No em dashes anywhere.** Full stop, colon, comma or middle dot.
- **Never invent proof.** Every claim traces to something real and checkable, or it does not ship.
- **Verify at 390px AND desktop in Ken's Brave through `claude-in-chrome`. Never Playwright, never
  headless Chrome.** A screenshot of each, or it is not verified.
- `npm run typecheck` and `npm run build` must be clean before you call anything done.

## B6 · The state of that repository, which you must not damage

**There is deployed but uncommitted work in the working tree from a previous session.** 21 code
files, 401 photographs re-signed with C2PA credentials, and 9 new files including
`scripts/sign-images.mjs`, `app/lib/attribution.ts` and `app/content/QuoteSimple.tsx`. It is live on
3d3d.ca and was left uncommitted deliberately, because that repository's law is commit only when
asked. Its own evidence note is at `docs/work-orders/evidence/EVIDENCE-2026-08-30-five-jobs.md`.

**So: the working tree matches the live site. Build on it. Do not revert it, do not stash it, do not
commit it, and do not run `git checkout` or `git restore` on anything you did not write.** When you
commit, name your own files explicitly. Never `git add -A` or `git add .` in that repository.

There is also a stale `wrangler dev` on port 8835 from that session. Leave it or use a different
port; do not assume 8835 is yours.

## B7 · Definition of done

- The hub renders at `/snitch`, below the tool, at 390px and at desktop, screenshotted in Ken's
  Brave through `claude-in-chrome`.
- Every link resolves. Open each one.
- The MCP config block copies to the clipboard and is valid JSON when pasted.
- No download button unless a release exists behind it, and if one does, the file it points at
  downloads.
- `npm run typecheck` and `npm run build` clean, output pasted.
- **Not deployed.** Ken reviews first. `npm run deploy` is his call, not yours.
- Your own files committed on a branch, named explicitly, with the 427 pre-existing changes
  untouched and still showing in `git status`.

## B8 · Report

Append a Part B section to your report, or write `/home/wess/3d3d-site/docs/EVIDENCE-2026-08-30-snitch-developer-hub.md`, containing:

1. What you built and where it sits in the page.
2. The two screenshots, 390px and desktop.
3. The typecheck and build output.
4. Every claim on the hub and what makes it true.
5. Anything you could not do, and why. If the download block is waiting on a release, say so plainly
   rather than shipping something that looks finished.
