# snitch

[![crates.io](https://img.shields.io/crates/v/snitch-tools?logo=rust&label=crates.io)](https://crates.io/crates/snitch-tools)
[![release](https://img.shields.io/github/v/release/3d3dcanada/snitch?logo=github)](https://github.com/3d3dcanada/snitch/releases/latest)
[![CI](https://github.com/3d3dcanada/snitch/actions/workflows/quality.yml/badge.svg)](https://github.com/3d3dcanada/snitch/actions/workflows/quality.yml)
[![MIT](https://img.shields.io/badge/licence-MIT-blue.svg)](LICENSE)

**Your photo is telling on you. Find out what, make it stop, or put your name on it instead.**

Written in Rust. Four binaries totalling 4.3 MB, no runtime to install, and it makes no network
call of any kind. It was Python until 2026-08-30, and that version is kept in `legacy/python/` as
the specification this one is checked against.

Three tools. You type the name of the thing you want.

```
snitch      photo.jpg    what is this file telling people about you
no-comment  photo.jpg    make it stop
credit      photo.jpg    put your name on it, so it stays
```

---

## snitch

```
$ snitch holiday.jpg

holiday.jpg  3,204,118 bytes
  LOCATION IS IN THIS FILE
    GPSLatitude      45.9636
    GPSLongitude     -66.6431
    Anyone who downloads this can see where it was taken.
    Make it stop:  no-comment holiday.jpg
  camera
    Camera           Google Pixel 9
    Taken            2026:07:14 18:22:41
  NO CREDIT AT ALL
    Nothing in this file says who made it.
    Fix it:  credit holiday.jpg --creator "Your Name"
  no C2PA Content Credential
```

It reads EXIF, IPTC, XMP and C2PA Content Credentials, and tells you whether a credential says the
image came from a generative model.

### The part nobody tells you

```
$ snitch --platforms

                 Visible stamp   C2PA Credentials   IPTC / XMP   EXIF
  LinkedIn       ? keeps         D partial           ? unknown    ? unknown
  Instagram      ? keeps         D partial           D partial    ? unknown
  Facebook       ? keeps         D partial           D partial    ? unknown
  X / Twitter    ? keeps         ? unknown           ? unknown    D STRIPS
  Reddit         ? keeps         ? unknown           ? unknown    D partial
  Printables     ? keeps         ? unknown           ? unknown    ? unknown
  Google Images  ? keeps         D partial           D reads      ? unknown
```

`D` means the platform documents the behaviour. `C` means independent upload/download tests
corroborate it. `?` means it is an explicitly unverified expectation, not a fact.

**Pixels are the only broadly portable layer.** LinkedIn documents C2PA display, but the rollout is
gradual and its handling of this tool's untrusted self-signed credentials has not been live-tested.
If credit matters, put it in the pixels and test your exact upload route.

`--notes` gives each cell's evidence class, limitation, and source URL. `--check` gives a repeatable
before/upload/download/after procedure. Platform behaviour can differ by feed, story, ad, message,
client, account, and file type, so corrections should include that context.

---

## no-comment

```
$ no-comment holiday.jpg
  holiday-clean.jpg  removed 24,118 bytes of metadata  pixels byte-identical
```

Drops private/application metadata while retaining JPEG JFIF, ICC colour, Adobe colour-transform,
and orientation data, plus PNG colour, transparency, orientation, and animation chunks. **This is
byte surgery, not re-encoding**, so every decoded frame comes out identical and the tool proves it
rather than asking you to take its word.

`--in-place` atomically replaces the input. `--out DIR` handles batches; existing outputs require
`--force`. In-place symlinks are refused rather than silently replacing the link.

---

## credit

```
$ credit shot.jpg \
    --creator "Jane Doe" \
    --credit "Doe Studio" \
    --copyright "© 2026 Doe Studio" \
    --licence cc-by-nc \
    --url https://example.com \
    --contact hello@example.com \
    --stamp "Doe Studio" --stamp-sub "example.com" --logo logo.png \
    --sign --digital-source camera
```

Writes the IPTC Core and XMP fields that picture desks and Google actually read: Creator, Credit,
CopyrightNotice, UsageTerms, WebStatement, LicensorName, LicensorURL, keywords.

**Strips GPS by default**, because a credit line should not come with your home address. Pass
`--keep-gps` if you want it.

- `--stamp` burns a visible mark into the pixels. **That is the only layer that survives
  screenshots**, though a platform can still crop or soften it. Stamping currently supports JPEG
  and PNG; it preserves existing PNG transparency.
- `--sign --digital-source camera` adds a development-grade self-signed C2PA Content Credential
  and requires an explicit source type so it never guesses camera versus AI provenance. It does
  not add a current CAWG identity assertion. LinkedIn documents inbound C2PA display, but rollout
  and untrusted self-signed handling remain unverified here.
- `--verify` checks an existing credential instead of writing anything.

Metadata-only credit has been exercised on JPEG, PNG, WebP, TIFF, and HEIC. Exact namespaces vary
by container and ExifTool support. Stamping is deliberately refused for WebP, TIFF, and HEIC rather
than writing a different format under the old extension.

`snitch --json FILE` emits a stable machine-readable report for scripts and batch checks.

Licence presets: `cc-by`, `cc-by-sa`, `cc-by-nd`, `cc-by-nc`, `cc-by-nc-sa`, `cc-by-nc-nd`, `cc0`,
`arr`.

---

## Install

Prebuilt binaries for Linux, macOS and Windows are on the
[releases page](https://github.com/3d3dcanada/snitch/releases). Download, unpack, put the three
commands on your PATH.

With a Rust toolchain:

```bash
cargo install snitch-tools
```

Or straight from the repository, to get whatever is on `main`:

```bash
cargo install --git https://github.com/3d3dcanada/snitch
```

Needs [ExifTool](https://exiftool.org):

```bash
sudo apt install libimage-exiftool-perl     # Debian, Ubuntu
brew install exiftool                       # macOS
choco install exiftool                      # Windows
```

Signing and full C2PA validation additionally need
[c2patool](https://github.com/contentauth/c2pa-rs): `cargo install c2patool`. Without it, `snitch`
reports C2PA validation as unavailable instead of falsely reporting that no credential exists;
ExifTool can still detect a C2PA/JUMBF container.

`credit --stamp` needs a system font to draw with. It looks for the usual ones and tells you to
pass `--font /path/to/font.ttf` if it cannot find any. On a minimal Linux image that usually means
`sudo apt install fonts-dejavu-core`.

### Build and test

```bash
cargo build --release
cargo test
cargo clippy --all-targets
cargo fmt --check
```

CI runs those on Linux, macOS and Windows.

---

## MCP server

The same tools, exposed to an AI assistant over a local stdio MCP server. It runs on your machine,
reads your files directly, and makes no network call of any kind.

`snitch-mcp` is installed alongside the other three. There is no `snitch mcp` subcommand, on
purpose: the name is the interface, and a protocol server is the last thing that should be buried
inside another command.

| Tool | Does |
| --- | --- |
| `snitch_inspect` | Everything a file is saying: GPS, camera, credit, PNG text chunks a generator wrote, C2PA credential |
| `snitch_strip_metadata` | Lossless strip to a new file, with a per-run decoded-pixel check. JPEG and PNG only |
| `snitch_add_credit` | Writes IPTC and XMP credit into a copy. Drops GPS unless you ask it not to |
| `snitch_verify_c2pa` | Presence, asset-binding integrity and signer trust, reported as three separate answers |
| `snitch_clean_text` | Finds and removes invisible tracking characters. Never touches ZWJ or ZWNJ |

Nothing is edited in place and no tool returns file bytes. A mutation tool returns the output path,
the byte delta, what class of thing was removed, and the honest state of its proof.

### Setup

Every host below takes the same block. Use the absolute path to `snitch-mcp`, which
`which snitch-mcp` will tell you.

**Claude Code** ([docs](https://code.claude.com/docs/en/mcp)), in `.mcp.json`:

```json
{
  "mcpServers": {
    "snitch": {
      "type": "stdio",
      "command": "/absolute/path/to/snitch-mcp"
    }
  }
}
```

or in one line:

```bash
claude mcp add --transport stdio snitch -- /absolute/path/to/snitch-mcp
```

**Claude Desktop** ([docs](https://modelcontextprotocol.io/docs/develop/connect-local-servers)), in
`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS or
`%APPDATA%\Claude\claude_desktop_config.json` on Windows:

```json
{
  "mcpServers": {
    "snitch": {
      "command": "/absolute/path/to/snitch-mcp"
    }
  }
}
```

**Cursor** ([docs](https://cursor.com/docs/context/mcp)), in `.cursor/mcp.json` for one project or
`~/.cursor/mcp.json` for all of them. Same block as Claude Desktop.

**Google Antigravity** ([docs](https://antigravity.google/docs/ide/mcp/)), in
`~/.gemini/config/mcp_config.json` globally or `.agents/mcp_config.json` in a workspace. Same block
again. In the IDE: the `…` menu at the top of the agent panel, MCP Servers, Manage MCP Servers,
View raw config.

---

## How it is built

Rust, and deliberately plain Rust.

| | |
| --- | --- |
| Source | 4,585 lines, 1,796 more in tests |
| Direct dependencies | **7** |
| Whole dependency tree | 47 crates |
| `async` / `tokio` anywhere in the tree | **0** |
| All four binaries, stripped | **4.3 MB** |
| `snitch-mcp` resident, idle | **2.4 MB** |
| Tests | 75 |

There is no async runtime, no web framework and no CLI framework. Arguments are parsed by hand and
MCP is a blocking read loop over newline-delimited JSON-RPC, because this is a single-operator tool
answering one request at a time over a pipe.

Two obvious dependencies were measured and refused, and the numbers are in `Cargo.toml`:

- **`c2pa`**, the Rust library behind c2patool, is 280 crates. c2patool is that library already
  compiled, and this shells out to it. Six times the tree to remove one subprocess is not a trade
  worth making.
- **`rmcp`**, the official Rust MCP SDK, is 59 crates and brings tokio. The protocol it implements
  is about two hundred lines against `serde_json`, which was already here.

The seven that earned their place: `serde` and `serde_json` for the JSON contract, `sha2` for the
pixel proof, `image` with only its jpeg and png features, `flate2` for the PNG text chunks,
`ab_glyph` to rasterise the stamp text, and `libc` on unix for two lines of `unsafe` that let a
closed pipe end the process quietly.

ExifTool stays a subprocess for the reason every serious tool in this space keeps it: nothing in
any language matches its tag database, and a narrower reader would quietly miss fields.

### It used to be Python

`legacy/python/` holds the original implementation, kept verbatim rather than deleted. It is the
specification the port is checked against: `snitch`, `no-comment` and `credit` produce identical
terminal output, identical JSON, identical exit codes and identical output files across the fixture
matrix. The platform table in `data/survival.json` was exported from it rather than retyped.

The Python `snitch-mcp` held 66.8 MB resident where this one holds 2.4 MB, measured on the same
machine in the same state.

---

## What these tools will not claim

Three things get overclaimed constantly in this space. None of them are true here.

**`no-comment` does not remove invisible watermarks.** In-pixel watermarks such as Google SynthID
are part of the image data. They survive re-encoding, cropping and resizing by design. The only
technique that touches them is diffusion regeneration, which repaints the image; the tools that do
it state they cannot detect whether it worked, and output remains classifiable as having been
through a removal pipeline. A 2025
[SynthID-Image evaluation](https://arxiv.org/abs/2510.09263) reported 99.72% true-positive
detection at 0.1% false positives for its external SynthID-O variant at its preferred resolution,
aggregated across the study's worst transformation settings.
`no-comment` removes **metadata**, including C2PA manifests. That is all it removes, and all it
says it removes.

**A self-signed credential does not prove who signed it.** The certificate generated on first use
produces a readable, tamper-evident development credential whose asset binding can validate. Its
creator block is a legacy metadata assertion, not a current CAWG identity assertion, and the
certificate is not on the C2PA trust list. A validator that checks identity therefore reports the
signer as unknown rather than as you. A conforming claim generator needs an eligible certificate
from a CA in the C2PA programme. Nothing here shortcuts that.

**Metadata does not create copyright.** You hold copyright in your work whether or not a file says
so. What metadata does is evidence authorship, carry your licence terms, and give an honest person
a way to credit you. That is genuinely useful, and it is a smaller claim than the one usually made.

---

## Why

Built by [3D3D](https://3d3d.ca) after a night spent putting provenance on a product and its
photographs, and finding that no single tool did the whole job, and that the tools claiming to do
the hard part were not doing it.

MIT licensed. Issues and corrections welcome, particularly on the platform table.
