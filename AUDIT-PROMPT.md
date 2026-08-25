# Audit prompt for Codex

Paste everything below the line into Codex, in the `~/snitch` directory.

---

You are auditing a small public Python CLI before it gets promoted and linked from a website. It is
live at https://github.com/3d3dcanada/snitch and installable, so bugs are already other people's
problem. Repo is at `~/snitch`. Be adversarial. Assume it is worse than it looks.

## What it is

Three commands for image credit and provenance:

- `snitch FILE` reads EXIF, IPTC, XMP and C2PA and reports what the file discloses, GPS loudly.
- `snitch --platforms` prints a table of what each platform keeps and strips on upload.
- `no-comment FILE` strips metadata by dropping JPEG `APPn`/`COM` segments or non-essential PNG
  chunks. Claims to be lossless and prints "pixels byte-identical" as proof.
- `credit FILE --creator ...` writes IPTC/XMP/EXIF, optionally burns a visible stamp into the
  pixels (`--stamp`), optionally signs a C2PA Content Credential (`--sign`).

Depends on ExifTool (required), c2patool (optional, only for `--sign`), Pillow.

## Your job

Audit for correctness, safety, portability and honesty, then fix what you find. Work in small
commits with a clear message each. Do not do a single giant rewrite.

**Verify empirically. Do not audit by reading alone.** Install it into a fresh venv, run every
command against real files, and check the output is actually true. There are real test images at
`~/Desktop/3D3D-STL/pencil holder/pictures/_originals/` (JPEG). Make your own PNGs, WebPs, TIFFs
and a HEIC if you can. Try a zero-byte file, a text file renamed `.jpg`, a symlink, a read-only
file, a path with spaces and unicode, a 100 MP image, and a file you do not have permission to
write.

## Known weak points. Start here, and do not assume this list is complete.

The previous author wrote these down honestly rather than making you find them:

1. **There are no tests. None.** That is the single biggest gap. Add a real suite with pytest,
   including round-trip tests: write credit, read it back, assert every field survives.
2. **The PNG path is close to untested.** `strip_png` and stamping a PNG were never exercised
   against a real PNG. Verify chunk handling, interlaced PNGs, palette PNGs, and PNGs with an alpha
   channel that the stamp compositing might wreck.
3. **`_JPEG_SKIPPABLE` strips APP0.** APP0 carries the JFIF header. Check whether removing it
   produces a file some decoders reject or render differently, and whether the `pixels_identical`
   check would even catch that. This may be a real bug.
4. **Pillow version floor is wrong.** `pyproject.toml` says `Pillow>=9.0` but the stamp code calls
   `ImageFont.load_default(size)`, which only takes a size argument from Pillow 10.1. On Pillow 9
   the stamp will silently render at the wrong size. Fix the floor or the call.
5. **`read_c2pa` has a nested conditional** that is hard to read and may be wrong:
   `if not os.path.exists(tool) if os.path.isabs(tool) else not shutil.which(tool)`. Work out what
   it actually evaluates to and rewrite it.
6. **Non-ASCII in IPTC.** The copyright symbol, accented names, non-Latin scripts. IPTC has legacy
   charset behaviour. Test that a name like `José Ángel` and a `©` survive a write-then-read
   round trip, in both IPTC and XMP.
7. **`--out` is silently ignored** for the second and subsequent files when several are passed.
   Either honour it as a directory, or refuse with an error. Silent is the wrong answer.
8. **`no-comment --in-place` writes `path + ".tmp"`** then moves. Not atomic if `/tmp` and the
   target are on different filesystems, and it collides if that name exists. Also: what happens if
   the process dies between write and move.
9. **Windows is unverified.** `~/.config/snitch` is wrong on Windows. Console colour codes are
   emitted whenever stdout is a tty, which may produce escape soup in some terminals.
10. **`pixels_identical` loads two full images into memory** with `tobytes()`. On a large file that
    is a lot of RAM for a check. Consider hashing in chunks.
11. **`sign` passes the private key through an environment variable**, which is readable via
    `/proc/PID/environ` by the same user. Assess whether c2patool offers a file-based alternative
    and whether this matters.
12. **Formats other than JPEG and PNG.** `no-comment` refuses them, which is honest, but `credit`
    will happily hand a HEIC or WebP to exiftool and may half-work. Decide the behaviour, make it
    consistent, and document it.
13. **No `--json` output.** Anything that wants to script `snitch` currently has to parse a
    human-formatted report.
14. **No CI, no linting, no type checking.** Add a GitHub Actions workflow that runs the tests on
    Linux and macOS, at minimum.

## The platform table needs specific scrutiny

`snitch/survival.py` asserts what LinkedIn, Instagram, Facebook, X, Reddit, Printables and Google
Images do to each metadata layer, and it is dated and presented as verified. **It was compiled from
research, not from the author uploading files to all seven platforms and downloading them back.**

That is a truth problem, because the README invites people to trust it and the website plans to
publish it as a reference.

Do this: for every cell, decide whether it is (a) documented by the platform, (b) widely
corroborated, or (c) inference. Mark anything that is (c) as unverified in the data structure
itself and render it differently, or remove it. **A table that is 90 % right and presented as
100 % verified is worse than a smaller table that is honest about its edges.** Do not silently
"correct" cells to what you believe; cite or mark.

## What NOT to change

- **The three command names.** `snitch`, `no-comment`, `credit` are the owner's decision. Do not
  rename, do not add an umbrella command, do not turn them back into subcommands.
- **The "what these tools will not claim" section** in the README, and the equivalent warnings in
  the skills and in `no-comment`'s output. Those are deliberate, they are correct, and they are the
  main thing separating this from the tools that overclaim. You may sharpen the wording. Do not
  soften or delete the substance.
- **GPS stripping is on by default in `credit`.** That is intentional.
- The MIT licence and the attribution.

## Report

When you finish, give me:

1. **What you found**, worst first, with the reproduction for each.
2. **What you fixed**, with the commit for each.
3. **What you could not verify and why.** Be explicit. "I could not test HEIC because I had no
   sample" is a useful sentence; silence is not.
4. **What you would do next** if you had another pass.
5. **Your honest read on whether this is fit to be promoted publicly right now**, and if not, the
   shortest list of things that would make it so.

Do not tell me it is production ready unless you have run it and believe that. If it is rough, say
it is rough.
