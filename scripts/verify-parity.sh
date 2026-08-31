#!/usr/bin/env bash
#
# Prove the Rust and the Python produce the same thing, on the same files, to the byte.
#
# THIS IS THE MOST IMPORTANT TEST IN THE REPOSITORY and it lived in a scratch directory until
# 2026-08-30, where it would have been lost the moment the machine was rebooted. `legacy/python/`
# is the specification: if these two disagree, the Python is presumed right until argued otherwise.
#
# Three defects were found by widening the fixture set this compares, and every one of them was
# invisible to a smaller set:
#
#   `no-comment` on a file with nothing to remove printed "already had nothing to remove" instead
#   of "removed 0 bytes of metadata  pixels byte-identical", quietly dropping the pixel proof.
#   Every earlier fixture carried metadata.
#
#   A file with no extension was refused as ". is not supported" rather than " is not supported",
#   because Python's splitext returns the extension WITH its dot.
#
#   A `credit` write failure lost its "FAILED" prefix and printed a full path where the Python
#   prints a basename.
#
# Usage:
#   scripts/verify-parity.sh                 # build, then compare
#   scripts/verify-parity.sh --no-build      # compare what is already built
#
# Needs: a Python with the legacy package installed, ExifTool, and cargo. c2patool is optional and
# its absence only means the C2PA fixtures compare as "no credential" on both sides, which is still
# a valid comparison.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/snitch-parity.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

RUST="$ROOT/target/release"
PY_BIN="${SNITCH_PYTHON_BIN:-}"

if [ "${1:-}" != "--no-build" ]; then
  echo "building the Rust release binaries"
  ( cd "$ROOT" && cargo build --release --locked ) || exit 1
fi

# The Python side. Point SNITCH_PYTHON_BIN at a venv's bin directory, or let this build one.
if [ -z "$PY_BIN" ]; then
  if command -v snitch >/dev/null 2>&1 && ! [ "$(command -v snitch)" -ef "$RUST/snitch" ]; then
    PY_BIN="$(dirname "$(command -v snitch)")"
  else
    echo "building a throwaway venv for the legacy Python"
    python3 -m venv "$WORK/venv" >/dev/null || exit 1
    "$WORK/venv/bin/pip" install --quiet -e "$ROOT/legacy/python" >/dev/null 2>&1 || {
      echo "could not install legacy/python; set SNITCH_PYTHON_BIN and rerun"; exit 1; }
    PY_BIN="$WORK/venv/bin"
  fi
fi
echo "python side: $PY_BIN"
echo "rust side:   $RUST"
echo

# ── the fixtures ──────────────────────────────────────────────────────────────────────────────
# Deliberately awkward. A matrix of ordinary photographs proves very little: three of the defects
# above hid behind fixtures that all carried metadata and all had normal extensions.
FIX="$WORK/fixtures"
mkdir -p "$FIX"
"$PY_BIN/python" - "$FIX" <<'PY'
import sys, os, shutil
from PIL import Image, PngImagePlugin
d = sys.argv[1]
p = lambda n: os.path.join(d, n)

Image.new("RGB", (64, 48), (120, 40, 200)).save(p("baseline.jpg"), quality=90)
Image.new("RGB", (64, 48), (120, 40, 200)).save(p("progressive.jpg"), quality=90, progressive=True)
Image.new("CMYK", (64, 48), (10, 20, 30, 40)).save(p("cmyk.jpg"), quality=90)
Image.new("L", (64, 48), 128).save(p("grayscale.jpg"), quality=90)
Image.new("RGB", (1, 1), (255, 0, 0)).save(p("onepixel.jpg"), quality=90)
Image.new("RGB", (64, 48), (90, 90, 90)).save(p("icc.jpg"), quality=90, icc_profile=b"fake icc profile")
Image.new("RGB", (64, 48), (10, 200, 90)).save(p("plain.png"))
Image.new("RGB", (64, 48), (10, 200, 90)).save(p("interlaced.png"), interlace=True)
Image.new("P", (64, 48)).save(p("palette.png"))
Image.new("RGBA", (64, 48), (10, 200, 90, 128)).save(p("alpha.png"))
Image.new("L", (64, 48), 200).save(p("gray.png"))
Image.new("I;16", (64, 48), 4000).save(p("sixteenbit.png"))
Image.new("RGB", (1, 1), (0, 255, 0)).save(p("onepixel.png"))

f1 = Image.new("RGB", (32, 32), (255, 0, 0))
f2 = Image.new("RGB", (32, 32), (0, 0, 255))
f1.save(p("animated.png"), save_all=True, append_images=[f2], duration=100, loop=0)

meta = PngImagePlugin.PngInfo()
meta.add_text("parameters", "a cat, (ugly hands:1.4)\nSteps: 30, Sampler: Euler a")
meta.add_text("workflow", '{"nodes":[{"id":1,"type":"KSampler"}]}')
meta.add_itxt("Description", "cafe naive 中文 \U0001F600")
Image.new("RGB", (24, 18), "purple").save(p("generated.png"), pnginfo=meta)

# the extension disagreeing with the content, and no extension at all
shutil.copy(p("baseline.jpg"), p("actually-a-jpeg.png"))
shutil.copy(p("plain.png"), p("actually-a-png.jpg"))
shutil.copy(p("baseline.jpg"), p("noextension"))
# a file that is not an image
open(p("broken.jpg"), "w").write("not an image at all, just text")
PY

# metadata on one, and every orientation the strip has to re-insert
exiftool -overwrite_original -q \
  -EXIF:Make="Canon" -EXIF:Model="EOS R5" -EXIF:Software="Parity Fixture" \
  -EXIF:DateTimeOriginal="2026:01:02 03:04:05" \
  -GPSLatitude=50.2447 -GPSLatitudeRef=N -GPSLongitude=-99.8433 -GPSLongitudeRef=W \
  -IPTC:By-line="Renée Åberg" -IPTC:CopyrightNotice="© 2026 Renée" \
  -XMP-dc:Creator="Renée Åberg" -XMP-dc:Description="café naïve 中文 😀" \
  -Comment="hello from a fixture" "$FIX/baseline.jpg" 2>/dev/null
for o in 2 3 4 5 6 7 8; do
  cp "$FIX/plain.png" "$FIX/orient$o.png"
  cp "$FIX/progressive.jpg" "$FIX/orient$o.jpg"
  exiftool -overwrite_original -q -Orientation#=$o "$FIX/orient$o.jpg" "$FIX/orient$o.png" 2>/dev/null
done

FIXTURES=$(cd "$FIX" && ls)
COUNT=$(echo "$FIXTURES" | wc -l)
echo "$COUNT fixtures"
echo

PASS=0
FAIL=0
KNOWN=0
FAILED=""

# Differences that are accounted for and accepted. The Python builds its usage block with argparse
# and the Rust prints its own, because the house style forbids a CLI framework. The error line
# beneath is byte-identical and so is the exit code, which is what a script greps for. Anything not
# on this list is a failure.
is_known_difference () {
  case "$1" in
    "credit nothing to do"|"credit --sign no source"|"credit --generated no sign") return 0 ;;
    *) return 1 ;;
  esac
}

note () { printf "  %-46s %s\n" "$1" "$2"; }

# ── snitch: read only, so both sides can look at the same file ────────────────────────────────
for f in $FIXTURES; do
  for mode in "" "--json"; do
    ( cd "$FIX" && NO_COLOR=1 "$PY_BIN/snitch" $mode "$f" >"$WORK/py.out" 2>&1; echo $? >"$WORK/py.rc" )
    ( cd "$FIX" && NO_COLOR=1 "$RUST/snitch"   $mode "$f" >"$WORK/rs.out" 2>&1; echo $? >"$WORK/rs.rc" )
    if diff -q "$WORK/py.out" "$WORK/rs.out" >/dev/null && \
       [ "$(cat "$WORK/py.rc")" = "$(cat "$WORK/rs.rc")" ]; then
      PASS=$((PASS + 1))
    else
      FAIL=$((FAIL + 1)); FAILED="$FAILED snitch${mode:+ $mode} $f"
      note "snitch ${mode} $f" "DIFFERS"; diff "$WORK/py.out" "$WORK/rs.out" | head -6
    fi
  done
done
for args in "--platforms" "--platforms --notes" "--platforms --check" \
            "--json --platforms" "--json --platforms --notes --check" ""; do
  NO_COLOR=1 "$PY_BIN/snitch" $args >"$WORK/py.out" 2>&1
  NO_COLOR=1 "$RUST/snitch"   $args >"$WORK/rs.out" 2>&1
  if diff -q "$WORK/py.out" "$WORK/rs.out" >/dev/null; then PASS=$((PASS + 1))
  else FAIL=$((FAIL + 1)); FAILED="$FAILED snitch $args"; note "snitch $args" "DIFFERS"
       diff "$WORK/py.out" "$WORK/rs.out" | head -6; fi
done
note "snitch" "$PASS compared"

# ── the writing commands: separate directories, then compare the produced bytes too ───────────
compare_write () {
  local label="$1"; local cmd="$2"; shift 2
  for side in py rs; do
    local d="$WORK/w-$side"
    rm -rf "$d"; mkdir -p "$d"
    for f in $FIXTURES; do cp "$FIX/$f" "$d/"; done
    local bin; [ "$side" = py ] && bin="$PY_BIN/$cmd" || bin="$RUST/$cmd"
    ( cd "$d" && NO_COLOR=1 XDG_CONFIG_HOME="$WORK/cfg-$side" "$bin" "$@" >_out.txt 2>&1; echo $? >_rc )
  done
  # the temporary filename in an ExifTool error is random on both sides by design
  sed -i -E 's/snitch-[A-Za-z0-9_]+/snitch-TMP/g; s#/w-(py|rs)/#/w-SIDE/#g' "$WORK/w-py/_out.txt" "$WORK/w-rs/_out.txt"
  if ! diff -q "$WORK/w-py/_out.txt" "$WORK/w-rs/_out.txt" >/dev/null || \
     [ "$(cat "$WORK/w-py/_rc")" != "$(cat "$WORK/w-rs/_rc")" ]; then
    if is_known_difference "$label"; then
      # The exit code still has to match, and so does the last line, which is the actual error.
      if [ "$(cat "$WORK/w-py/_rc")" = "$(cat "$WORK/w-rs/_rc")" ] && \
         [ "$(tail -1 "$WORK/w-py/_out.txt")" = "$(tail -1 "$WORK/w-rs/_out.txt")" ]; then
        KNOWN=$((KNOWN + 1)); note "$label" "known difference: argparse usage block, error line identical"
        return
      fi
      FAIL=$((FAIL + 1)); FAILED="$FAILED $label(known difference grew)"
      note "$label" "A KNOWN DIFFERENCE CHANGED SHAPE"
      diff "$WORK/w-py/_out.txt" "$WORK/w-rs/_out.txt" | head -8
      return
    fi
    FAIL=$((FAIL + 1)); FAILED="$FAILED $label"
    note "$label" "OUTPUT DIFFERS"; diff "$WORK/w-py/_out.txt" "$WORK/w-rs/_out.txt" | head -8
    return
  fi
  local bad=""
  for produced in $(cd "$WORK/w-py" && ls | grep -vE '^_'); do
    cmp -s "$WORK/w-py/$produced" "$WORK/w-rs/$produced" || bad="$bad $produced"
  done
  if [ -n "$bad" ]; then
    FAIL=$((FAIL + 1)); FAILED="$FAILED $label(bytes:$bad)"; note "$label" "BYTES DIFFER:$bad"
  else
    PASS=$((PASS + 1))
  fi
}

for f in $FIXTURES; do
  compare_write "no-comment $f" no-comment "$f"
done
compare_write "no-comment --in-place"     no-comment --in-place baseline.jpg
compare_write "no-comment -o"             no-comment -o out.jpg baseline.jpg
compare_write "no-comment missing"        no-comment missing.jpg
for f in $FIXTURES; do
  compare_write "credit $f" credit "$f" --creator "Renée Åberg" --copyright "© 2026"
done
compare_write "credit --licence"          credit baseline.jpg --creator X --licence cc-by
compare_write "credit --keep-gps"         credit baseline.jpg --creator X --keep-gps
compare_write "credit --keyword"          credit baseline.jpg --creator X --keyword a --keyword b
compare_write "credit --in-place"         credit baseline.jpg --creator X --in-place
compare_write "credit nothing to do"      credit baseline.jpg
compare_write "credit --verify"           credit --verify baseline.jpg
compare_write "credit bad licence"        credit baseline.jpg --creator X --licence nope

echo
if [ "$FAIL" -eq 0 ]; then
  echo "PARITY HOLDS: $PASS comparisons byte for byte, exit codes included."
  if [ "$KNOWN" -gt 0 ]; then
    echo "$KNOWN accounted-for difference(s), each one argparse's usage block above an identical error line."
  fi
  exit 0
fi
echo "PARITY BROKEN: $PASS passed, $FAIL failed."
echo "failing:$FAILED"
echo
echo "legacy/python is the specification. If the Rust is right and the Python is wrong, say so"
echo "explicitly in the commit rather than changing this script to agree with the Rust."
exit 1
