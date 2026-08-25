# Final verification — 2026-08-25

Final scratch root: `/tmp/snitch-final-20260825.rks92L`. Final verification used a newly created
venv and a second new venv containing only the built wheel and its runtime dependency.

## Toolchain

```text
Python 3.12.3
pip 26.2.1
Pillow 12.3.0
ExifTool 12.76
c2patool 0.27.15
OpenSSL 3.0.13
Ruff 0.16.4
mypy 1.20.2
pytest 9.1.1
build 1.5.0
Twine 7.0.0
```

## Quality and package gates

```text
$ ruff check snitch tests
All checks passed!

$ mypy snitch
Success: no issues found in 6 source files

$ pytest -q
..............................................................           [100%]
62 passed in 4.09s

$ python -m build --outdir /tmp/snitch-final-20260825.rks92L/dist
Successfully built snitch_tools-0.1.0.tar.gz and snitch_tools-0.1.0-py3-none-any.whl

$ python -m twine check /tmp/snitch-final-20260825.rks92L/dist/*
wheel: PASSED
sdist: PASSED

$ go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.12 .github/workflows/quality.yml
exit 0
```

Wheel metadata reported `Requires-Python: >=3.9` and `Requires-Dist: Pillow>=10.1`. A clean wheel
venv installed Pillow 12.3.0 and snitch-tools 0.1.0. `snitch`, `no-comment`, `credit`, and
`python -m snitch` each printed version 0.1.0. Platform output captured through a pipe contained
zero ESC bytes.

## Fresh installed-wheel behavior

The real JPEG was copied from the requested `_originals` directory into a path containing spaces
and Unicode. The first smoke script guessed a stale filename (`20250809_163945.jpg`) and stopped at
`cp: cannot stat`; it verified no product behavior. The rerun resolved the actual UUID `.jpeg`
fixture and completed from the beginning.

`snitch --json` identified the real image as `image/jpeg` with no C2PA credential. `no-comment`
exited zero, reported byte-identical pixels, and produced a decodable JPEG. Metadata-only `credit`
exited zero and an independent JSON inspection returned the exact values `José Ángel`,
`Atelier Étoile`, `© 2026 Atelier Étoile — 東京`, and `Final real-file proof`.

ExifTool reported the same decoded image-data hash for the source, stripped output, and credited
output:

```text
ad7f6f030b7e185da24c796946b5ba4b
```

The platform JSON report parsed successfully and every one of its 28 cells had
`live_tested: false`.

A fresh Unicode self-signed credential was generated with a 0600 private key. `credit --verify`
returned zero for the untouched asset and printed:

```text
Valid  certificate issuer Atelier Étoile
SIGNER IDENTITY UNTRUSTED (certificate not on validator trust list)
```

The detailed validator report recorded claim generator and software agent as `snitch 0.1.0`,
Digital Source Type `humanEdits`, issuer `Atelier Étoile`, and validation state `Valid`. After an
ExifTool comment was added, validation state became `Invalid` and the CLI exited 1. An unsigned
real JPEG also exited 1.

## Explicitly unverified

- No authenticated upload/download was made to LinkedIn, Instagram, Facebook, X, Reddit,
  Printables, Google, or any proposed additional platform.
- The authored GitHub Actions matrix has not run because nothing was pushed. macOS, Windows,
  CPython 3.9, and CPython 3.14 therefore remain locally unobserved.
- No HEIC was generated from a camera in this run. A real local HEIC sample was metadata-credited
  and read successfully earlier in the same audit; that one sample does not prove every HEIF
  variant.
- No trusted C2PA certificate, CAWG identity credential, KMS/HSM, or conformance-program product
  was available. The self-signed path is development-grade only.
- No platform account action, purchase, push, release, publication, or deployment was performed.

The worktree at close contained only the user's pre-existing untracked `AUDIT-KICKOFF.txt` and
`AUDIT-PROMPT.md`; neither was staged or committed.
