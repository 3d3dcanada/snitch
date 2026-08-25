# Known weak points — observed closure

All 14 items in section 3 of `AUDIT-PROMPT.md` were exercised. Commands below ran from the clean
audit venv under `/tmp/snitch-audit-20260825.px7b6I`; generated outputs stayed under that scratch
root. The final suite contains 62 passing tests, including real `c2patool` and OpenSSL round trips
when those tools are installed.

| # | Observed result and disposition | Commit(s) |
|---:|---|---|
| 1 | There were no tests. Added unit, CLI, malformed-container, round-trip, and live C2PA tests. Unicode credit is asserted separately in EXIF, IPTC, and XMP rather than only through the merged report. | `886c41f` through `ab82253` |
| 2 | Live RGB, RGBA, palette/transparency, ImageMagick-interlaced, and animated PNGs were stripped and decoded. Stamping had destroyed alpha; it now preserves RGBA/transparency. CRC, length, ordering, and critical-chunk corruption are rejected without output. | `886c41f`, `ef57fb9` |
| 3 | The original JPEG stripper removed JFIF APP0. The result decoded in Pillow, ImageMagick, `file`, and ffmpeg, but that does not prove universal compatibility. JFIF/JFXX, Adobe APP14, ICC APP2, and orientation-only EXIF are now retained. CMYK and ICC/orientation fixtures remain display-equivalent and pixel-identical. | `886c41f` |
| 4 | A clean Pillow 10.0.1 venv raised `TypeError: load_default() takes 0 positional arguments but 1 was given`. The declared floor is now Pillow 10.1. | `ef57fb9` |
| 5 | The nested `read_c2pa` conditional was replaced by an explicit resolver. Missing validator, absent credential, invalid asset, and validator error are distinct states. | `4e33d38` |
| 6 | Before the fix, `José Ángel`, `©`, and non-Latin IPTC text became `?` while XMP remained correct and hid the loss in the merged report. ExifTool now receives an explicit UTF-8 IPTC charset. Live JPEG/PNG round trips preserved `José Ángel`, `© café`, and `東京` in the supported namespaces. | `ef57fb9` |
| 7 | Multi-file `--out` ignored the requested path. It now requires an existing directory, places every output there, rejects a regular output file, detects collisions, and requires `--force` before replacement. | `970e738`, `ef57fb9` |
| 8 | In-place writes used a predictable `.tmp`, overwrote a sentinel, and replaced symlinks. They now use a unique same-directory sibling and `os.replace`, and refuse in-place symlinks. A forced SIGKILL during a 100 MP run left the target SHA-256 and metadata unchanged; it did leave one orphan sibling because a killed process cannot run cleanup. The orphan was removed after observation. | `970e738`, `ef57fb9` |
| 9 | Config paths now follow `%APPDATA%` on Windows, `~/Library/Application Support` on macOS, and XDG on Linux. ANSI is disabled for pipes, `NO_COLOR`, and conservatively on Windows. Unit tests cover the branches and CI covers all three OS families, but only Linux was live-tested locally and the workflow has not run remotely. | `4e33d38`, `a4ec575`, `df4a12f` |
| 10 | `/usr/bin/time -v` measured 1,679,800 KiB maximum RSS for the original 100 MP pixel proof. Sequential frame hashing and chunked reads reduced the observed maximum to 417,648 KiB; the cleaned image remained pixel-identical. This is much better, not small. | `886c41f` |
| 11 | Signing no longer passes private material in environment variables. It passes absolute key/certificate file paths and removes inherited `C2PA_PRIVATE_KEY` and `C2PA_SIGN_CERT`. The generated private key was mode 0600. Current CAI guidance still says filesystem keys are development-only and production should use KMS/HSM-backed signing. | `4e33d38` |
| 12 | Metadata-only credit was live-tested on JPEG, PNG, WebP, TIFF, and a real HEIC sample; `ImageDataHash` stayed equal. Visible stamping is now restricted to JPEG/PNG. The previous WebP/TIFF path wrote JPEG bytes under the old extension; all unsupported stamp attempts now fail before creating an artifact. | `ef57fb9` |
| 13 | `snitch --json FILE` and JSON platform reports now provide stable Unicode machine output, including per-file errors and evidence metadata. | `a4ec575`, `31404bc` |
| 14 | Added Ruff, mypy, pytest, isolated build/Twine/wheel smoke checks, and pinned GitHub Actions for Ubuntu Python 3.9 and 3.14, macOS, and Windows. `actionlint` passed locally. The GitHub matrix remains unobserved until this branch is pushed. | `df4a12f` |

## Selected live checks

Metadata-only credit on JPEG, PNG, WebP, TIFF, and HEIC exited zero. ExifTool `ImageDataHash` matched
before and after for all five. `snitch --json` returned the exact Unicode creator/copyright values.

The JPEG and four PNG stripping fixtures were decoded with Pillow, ImageMagick, and ffmpeg. Raw
decoded hashes matched before/after. Adobe RGB profiles and orientation 6 survived while private
artist data was removed. A CMYK JPEG remained CMYK with the same ExifTool image-data hash.

Visible JPEG, RGBA PNG, and palette PNG stamps exited zero. ImageMagick's absolute-error metric
observed changed pixels, transparent corners remained transparent, and ffmpeg decoded each output.
WebP, TIFF, HEIC, and a missing logo exited nonzero and left zero output artifacts.

The live self-signed C2PA round trip validated its asset binding. Adding an ExifTool comment after
signing produced `assertion.dataHash.mismatch`; `credit --verify` now exits 1 for that invalid
credential and for an unsigned file. Human output labels the certificate issuer and prints
`SIGNER IDENTITY UNTRUSTED`.

Zero-byte, renamed-text, missing, directory, locked-directory, symlink, collision, and option-like
filename probes all returned nonzero where appropriate and did not overwrite the source. Missing
ExifTool is now a one-line dependency error for both inspection and credit writing, with no
traceback or partial artifact.
