# WO-01 baseline evidence

Run on 2026-08-25 from commit `2f774b8` on branch
`audit/pre-publication-2026-08-25`. Scratch root:
`/tmp/snitch-audit-20260825.px7b6I`.

## Clean installation

```text
$ python3 -m venv .../venv
$ .../venv/bin/python -m pip install /home/wess/snitch
Successfully built snitch-tools
Successfully installed Pillow-12.3.0 snitch-tools-0.1.0
$ exiftool -ver
12.76
$ c2patool --version
c2patool 0.27.15
```

All three installed entry points returned their help successfully. `python -m snitch` did not:

```text
No module named snitch.__main__; 'snitch' is a package and cannot be directly executed
exit=1
```

## Real and generated fixtures

- Real 2048x1536 JPEG copied from the requested `_originals` directory.
- RGB, RGBA, palette/transparency, and ImageMagick-confirmed interlaced PNGs.
- Pillow-generated WebP and TIFF.
- Real HEIC sample found locally at
  `/home/wess/go/pkg/mod/github.com/wailsapp/mimetype@v1.4.1/testdata/heic.single.heic`.
- Zero-byte JPEG, UTF-8 text renamed `.jpg`, symlink, locked directory/read-only JPEG, spaces and
  Unicode in paths, and a generated 10000x10000 (100 MP) JPEG.

## Observed command behaviour before fixes

`snitch`:

- Correctly read the real JPEG and a fully credited JPEG.
- Reported both a zero-byte file and text renamed `.jpg` as valid-looking images with no location,
  no credit, and no credential; both exited 0.
- A missing path also exited 0.
- `--platforms --notes --check` labelled the entire table `verified 2026-08-25`, despite the audit
  prompt confirming that the seven platform round trips were never performed.
- Raw DIM escape sequences appeared even though output was captured through a non-TTY pipe.

`no-comment`:

- JPEG, RGB PNG, RGBA PNG, palette PNG, and interlaced PNG strips all decoded successfully and
  reported `pixels byte-identical`.
- It removed the JPEG JFIF APP0 segment. Pillow, ImageMagick, `file`, and later ffmpeg checks could
  still decode the fixture, but this does not prove universal decoder compatibility.
- Zero-byte and renamed-text errors still exited 0.
- `--out requested-output file1 file2` silently ignored the requested path, created two default
  `-clean` files, and exited 0.
- `--in-place` in a locked directory raised an uncaught `PermissionError` traceback.
- A pre-existing `file.jpg.tmp` sentinel was overwritten and then removed.
- In-place operation on a symlink replaced the link with a regular file and did not change its
  target.
- The 100 MP strip passed, but `/usr/bin/time -v` measured 1,679,800 KiB maximum RSS and Pillow
  emitted a decompression-bomb warning. This confirms the full-image comparison is costly.

`credit`:

- A full ASCII/Unicode round trip appeared correct through `snitch`, but namespace-level ExifTool
  output showed every non-ASCII IPTC value replaced by `?`; XMP remained correct and masked the
  corruption in the human report.
- GPS added to the source was removed by default.
- `--out` was silently ignored for multiple inputs, as in `no-comment`.
- Calling `credit FILE` with no credit fields printed `credit written ok`; a follow-up `snitch`
  reported `NO CREDIT AT ALL`.
- Metadata-only credit round-tripped on live JPEG, PNG, WebP, TIFF, and HEIC samples.
- Stamping an RGBA PNG converted it from RGBA with alpha extrema `(0, 156)` to RGB, destroying
  transparency. A palette PNG likewise lost its transparency.
- Stamping WebP or TIFF wrote JPEG bytes beneath `.webp`/`.tiff`, ExifTool rejected the result, and
  the command still exited 0.
- A nonexistent `--logo` was silently treated as a successful visible stamp.
- Locked, zero-byte, and renamed-text writes printed `FAILED` but exited 0 and left copied outputs
  for the latter two cases.
- Signing and verification ran successfully with a scratch P-256 key, but an accented organisation
  appeared as `Atelier Ãtoile` in the issuer and the default manifest asserted `digitalCapture`
  without evidence that the input came from a camera.
- In-place operation on a symlink replaced the link with a regular credited file and did not change
  its target.

## Dependency-floor reproduction

A second clean venv installed the allowed Pillow 10.0.1. Its live signature was
`ImageFont.load_default()`; `ImageFont.load_default(50)` raised:

```text
TypeError load_default() takes 0 positional arguments but 1 was given
```

The existing broad exception handler then produced a stamp with the old fixed-size default font,
confirming the declared floor silently defeats stamp scaling.
