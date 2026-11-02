#!/usr/bin/env bash
# Build the merged Korean craft-fonts input for PdfCraft, from public git refs only:
#
#   storytold/craft-fonts @ b076417  (the shared faces, the Japanese and Simplified-Chinese
#                                     manifests, NanumGothic)
#   + homeass/craft-fonts feat/hakgyoansim-namu  (Namu Regular/Light, Sangjang R, Chulseokbu L/B)
#   + homeass/craft-fonts feat/tinos-ui-serif   (Tinos Regular/Bold/Italic/BoldItalic)
#
# The added font directories are extracted with `git archive`; their manifest rows are appended
# to the base `fonts/manifest.txt`. The base's `fonts/simplified-chinese-manifest.txt` is copied
# untouched (`crates/fonts/build.rs` requires it, and an exact Hans face within 16 MiB, when
# CRAFT_FONTS_REQUIRED is set). The refs are pinned commits, so the output is reproducible.
#
# Usage: packaging/craft-fonts-korean-merge.sh [-o DIR]
#
#   -o, --out DIR   output checkout (default: ./craft-fonts-korean). An existing checkout made
#                   by this script is replaced; any other non-empty directory is refused.
#
# Then: CRAFT_FONTS_DIR=<DIR> CRAFT_FONTS_REQUIRED=1 cargo check -p pdfcraft-fonts
#
# Needs: bash, git, tar and the network (two clones: storytold/craft-fonts and
# homeass/craft-fonts). No cargo, and no Python/Perl: it runs on Linux, macOS and Git Bash.
set -euo pipefail

BASE_REPO=https://github.com/storytold/craft-fonts.git
BASE_COMMIT=b076417b5880bd4154bd22f00695b1d28c6bea83
KOREAN_REPO=https://github.com/homeass/craft-fonts.git
NAMU_COMMIT=a8b44dec3141b57dda3144a41b50bf403b4f62ff # feat/hakgyoansim-namu
TINOS_COMMIT=44d960d43e5c8fff2cba758d6f69f4d70f69a46e # feat/tinos-ui-serif

# What the pinned refs produce. A change here means a ref moved, which this script must not
# silently accept: the release's embedded faces would change under the same label.
BASE_FACES=13
APPENDED_FACES=9
EXPECTED_FACES=22
EXPECTED_KORE=6 # NanumGothic (base) + Namu R/L + Sangjang R + Chulseokbu L/B
EXPECTED_TINOS=4
MAX_HANS_BYTES=$((16 * 1024 * 1024))

OUT=./craft-fonts-korean

USAGE='
Usage: packaging/craft-fonts-korean-merge.sh [-o DIR]

  -o, --out DIR   output checkout (default: ./craft-fonts-korean)

Builds the merged Korean craft-fonts input from pinned storytold/craft-fonts and
homeass/craft-fonts commits, then verifies every manifest face and licence file.'

die() {
  echo "error: $*" >&2
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -o | --out)
      [[ $# -ge 2 ]] || die "$1 needs a directory"
      OUT="$2"
      shift 2
      ;;
    --out=*)
      OUT="${1#--out=}"
      shift
      ;;
    -h | --help)
      echo "$USAGE"
      exit 0
      ;;
    *)
      die "unknown argument: $1 (try --help)"
      ;;
  esac
done
[[ -n "$OUT" ]] || die "-o needs a non-empty directory"
for tool in git tar; do command -v "$tool" >/dev/null 2>&1 || die "$tool is required"; done

mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
# Replace only a directory this script made (or an empty one). Anything else may be a real
# checkout the caller pointed at by mistake.
if [[ -n "$(ls -A "$OUT" 2>/dev/null)" ]]; then
  if [[ -f "$OUT/fonts/manifest.txt" ]]; then
    echo "==> replacing the existing font checkout at $OUT"
    rm -rf "${OUT:?}/fonts"
  else
    die "$OUT is not empty and does not look like a craft-fonts checkout; remove it or choose another -o"
  fi
fi

TMP="$(mktemp -d "${TMPDIR:-/tmp}/craft-fonts-korean.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

# Fetch one pinned commit into an empty repository and leave it as FETCH_HEAD.
fetch_commit() {
  local repo="$1" commit="$2" dest="$3" got
  mkdir -p "$dest"
  git -C "$dest" init -q
  git -C "$dest" remote add origin "$repo"
  if ! git -C "$dest" fetch -q --depth 1 origin "$commit"; then
    die "could not fetch $commit from $repo"
  fi
  got="$(git -C "$dest" rev-parse FETCH_HEAD)"
  [[ "$got" == "$commit" ]] || die "$repo: asked for $commit, got $got"
}

# Extract paths from the repository's FETCH_HEAD.
extract() {
  local repo="$1" dest="$2"
  shift 2
  git -C "$repo" archive --format=tar FETCH_HEAD -- "$@" | tar -x -C "$dest"
}

# Face rows of the current FETCH_HEAD manifest under the given font paths.
face_rows() {
  local repo="$1" pattern="$2" dest="$3"
  git -C "$repo" show FETCH_HEAD:fonts/manifest.txt | grep -E "[|] ${pattern}" >"$dest" || true
}

count_faces() {
  local file="$1" n=0 line
  while IFS= read -r line; do
    [[ "$line" =~ ^[[:space:]]*$ ]] && continue
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    n=$((n + 1))
  done <"$file"
  echo "$n"
}

trim() {
  local s="$1"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "$s"
}

has_script() {
  case ",$1," in
    *",$2,"*) return 0 ;;
    *) return 1 ;;
  esac
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

echo "==> base: $BASE_REPO @ $BASE_COMMIT"
fetch_commit "$BASE_REPO" "$BASE_COMMIT" "$TMP/base"
extract "$TMP/base" "$OUT" fonts
[[ -f "$OUT/fonts/manifest.txt" ]] || die "the base checkout has no fonts/manifest.txt"
[[ -f "$OUT/fonts/simplified-chinese-manifest.txt" ]] || die "the base checkout has no fonts/simplified-chinese-manifest.txt"
BASE_FACE_COUNT="$(count_faces "$OUT/fonts/manifest.txt")"
[[ "$BASE_FACE_COUNT" -eq "$BASE_FACES" ]] || die "base manifest has $BASE_FACE_COUNT faces, expected $BASE_FACES"

echo "==> Korean: $KOREAN_REPO @ $NAMU_COMMIT"
fetch_commit "$KOREAN_REPO" "$NAMU_COMMIT" "$TMP/korean"
extract "$TMP/korean" "$OUT" \
  fonts/hakgyoansim-namu fonts/hakgyoansim-sangjang fonts/hakgyoansim-chulseokbu
face_rows "$TMP/korean" 'fonts/hakgyoansim-(namu|sangjang|chulseokbu)/' "$TMP/namu-rows.txt"
NAMU_ROWS="$(wc -l <"$TMP/namu-rows.txt" | tr -d ' ')"
[[ "$NAMU_ROWS" -eq 5 ]] || die "expected 5 Hakgyoansim manifest rows, found $NAMU_ROWS"

echo "==> Tinos: $KOREAN_REPO @ $TINOS_COMMIT"
fetch_commit "$KOREAN_REPO" "$TINOS_COMMIT" "$TMP/tinos"
extract "$TMP/tinos" "$OUT" fonts/tinos
face_rows "$TMP/tinos" 'fonts/tinos/' "$TMP/tinos-rows.txt"
TINOS_ROWS="$(wc -l <"$TMP/tinos-rows.txt" | tr -d ' ')"
[[ "$TINOS_ROWS" -eq 4 ]] || die "expected 4 Tinos manifest rows, found $TINOS_ROWS"
[[ "$((NAMU_ROWS + TINOS_ROWS))" -eq "$APPENDED_FACES" ]] || die "expected $APPENDED_FACES appended faces, found $((NAMU_ROWS + TINOS_ROWS))"

# A row whose file is already in the base manifest would embed the face twice.
while IFS= read -r row; do
  file="$(printf '%s\n' "$row" | awk -F' [|] ' '{ gsub(/^[ \t]+|[ \t]+$/, "", $3); print $3 }')"
  if grep -Fq "| $file |" "$OUT/fonts/manifest.txt"; then
    die "the base manifest already names $file; refusing to add a duplicate face"
  fi
done <"$TMP/namu-rows.txt"
while IFS= read -r row; do
  file="$(printf '%s\n' "$row" | awk -F' [|] ' '{ gsub(/^[ \t]+|[ \t]+$/, "", $3); print $3 }')"
  if grep -Fq "| $file |" "$OUT/fonts/manifest.txt"; then
    die "the base manifest already names $file; refusing to add a duplicate face"
  fi
done <"$TMP/tinos-rows.txt"

cat "$TMP/namu-rows.txt" "$TMP/tinos-rows.txt" >>"$OUT/fonts/manifest.txt"

# ---- self-check --------------------------------------------------------------------------------
MANIFEST="$OUT/fonts/manifest.txt"
faces=0
kore=0
tinos=0
while IFS= read -r line; do
  [[ "$line" =~ ^[[:space:]]*$ ]] && continue
  [[ "$line" =~ ^[[:space:]]*# ]] && continue
  faces=$((faces + 1))
  IFS='|' read -r family style file scripts licence licence_file hash _source <<<"$line"
  family="$(trim "$family")"
  style="$(trim "$style")"
  file="$(trim "$file")"
  scripts="$(trim "$scripts")"
  licence="$(trim "$licence")"
  licence_file="$(trim "$licence_file")"
  hash="$(trim "$hash")"
  [[ -n "$family" && -n "$style" && -n "$file" && -n "$scripts" ]] || die "malformed manifest row: $line"
  [[ "$licence" == "OFL-1.1" ]] || die "$family $style: expected OFL-1.1, found $licence"
  [[ -f "$OUT/$file" ]] || die "$family $style: manifest names a missing font: $file"
  [[ -f "$OUT/$licence_file" ]] || die "$family $style: manifest names a missing licence: $licence_file"
  actual="$(sha256_of "$OUT/$file")"
  [[ "$actual" == "$hash" ]] || die "$family $style: sha256 of $file is $actual, manifest says $hash"
  if has_script "$scripts" Kore; then kore=$((kore + 1)); fi
  if [[ "$family" == "Tinos" ]]; then tinos=$((tinos + 1)); fi
done <"$MANIFEST"

[[ "$faces" -eq "$EXPECTED_FACES" ]] || die "merged manifest has $faces faces, expected $EXPECTED_FACES"
[[ "$kore" -eq "$EXPECTED_KORE" ]] || die "merged manifest has $kore Kore rows, expected $EXPECTED_KORE"
[[ "$tinos" -eq "$EXPECTED_TINOS" ]] || die "merged manifest has $tinos Tinos rows, expected $EXPECTED_TINOS"

# The required native Chinese manifest must still add an exact Hans face within build.rs's cap.
hans=0
while IFS= read -r line; do
  [[ "$line" =~ ^[[:space:]]*$ ]] && continue
  [[ "$line" =~ ^[[:space:]]*# ]] && continue
  IFS='|' read -r _family _style file scripts _licence _licence_file _hash _source <<<"$line"
  scripts="$(trim "$scripts")"
  file="$(trim "$file")"
  if has_script "$scripts" Hans; then
    [[ -f "$OUT/$file" ]] || die "the Simplified-Chinese manifest names a missing font: $file"
    bytes="$(wc -c <"$OUT/$file" | tr -d ' ')"
    [[ "$bytes" -le "$MAX_HANS_BYTES" ]] || die "$file is $bytes bytes, over the $MAX_HANS_BYTES-byte Hans cap"
    hans=$((hans + 1))
  fi
done <"$OUT/fonts/simplified-chinese-manifest.txt"
[[ "$hans" -ge 1 ]] || die "the Simplified-Chinese manifest has no exact Hans face"

echo "==> $OUT"
echo "manifest face lines: $faces ($BASE_FACE_COUNT base + $((NAMU_ROWS + TINOS_ROWS)) appended)"
echo "Kore rows: $kore"
echo "Tinos rows: $tinos"
echo "Hans faces (native Chinese manifest): $hans"
echo "verified: every manifest face and licence file exists, and every sha256 matches"
