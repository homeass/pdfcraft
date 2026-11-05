#!/usr/bin/env bash
# Build the Korean craft-i18n input for PdfCraft: the pinned upstream catalogs
# (storytold/craft-i18n at the SHA in `craft-i18n.rev`) plus the four Korean face-label rows
# the fork's font pickers show, taken verbatim from the catalog the chain carried in-repo
# before upstream moved the catalogs to craft-i18n.
#
#   storytold/craft-i18n @ <craft-i18n.rev>   (every locale, unchanged)
#   + locales/ko/pdfcraft.tsv                 (the four rows below, inserted in catalog order)
#
# `Korean -> 한국어` already exists in upstream's `locales/ko/common.tsv`, so it is not
# re-added here. Nothing else is changed; the upstream url and rev stay as pinned, and the
# flatpak `commit:` pins keep pointing at the same SHA (their test stays green).
#
# Usage: packaging/craft-i18n-korean-merge.sh [-o DIR]
#
#   -o, --out DIR   output checkout (default: ./craft-i18n-korean). An existing checkout made
#                   by this script is replaced; any other non-empty directory is refused.
#
# Then: CRAFT_I18N_DIR=<DIR> cargo build
#
# `crates/ui-egui/build.rs` embeds `<DIR>/locales/<code>/pdfcraft.tsv`; the path must be
# absolute and CRAFT_I18N_REQUIRED=1 makes a missing input a build error. Unset, the app is
# English only. The same variable drives the catalog tests.
#
# Needs: bash, git, tar, awk and the network (one pinned fetch). No cargo, and no
# Python/Perl: it runs on Linux, macOS and Git Bash.
set -euo pipefail

REPO=https://github.com/storytold/craft-i18n.git
APP=pdfcraft
LOCALE=ko
# The rows the chain added to its in-repo ko.tsv (commit "ui: name the Korean face each picker
# family writes Hangul in"), as (English source text, Korean translation). Empty context.
ROWS=4

OUT=./craft-i18n-korean

USAGE='
Usage: packaging/craft-i18n-korean-merge.sh [-o DIR]

  -o, --out DIR   output checkout (default: ./craft-i18n-korean)

Builds the Korean craft-i18n input from the pinned storytold/craft-i18n commit in
craft-i18n.rev, adds the four Korean face-label rows to locales/ko/pdfcraft.tsv, then
checks the catalog structure and that every row resolves.'

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
for tool in git tar awk; do command -v "$tool" >/dev/null 2>&1 || die "$tool is required"; done

# The rev lives in the branch, not in this script: read it, never hardcode it. `git rev-parse`
# also works when the workflow copied this file to a temporary directory to normalize its line
# endings (the working directory is still the checkout).
repo_root() {
  git rev-parse --show-toplevel 2>/dev/null || (cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
}
ROOT="$(repo_root)"
REV_FILE="$ROOT/craft-i18n.rev"
[[ -f "$REV_FILE" ]] || die "no craft-i18n.rev at $REV_FILE"
REV="$(tr -d '[:space:]' <"$REV_FILE")"
[[ "$REV" =~ ^[0-9a-f]{40}$ ]] || die "craft-i18n.rev must hold one 40-character SHA, found '$REV'"

mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
# Replace only a directory this script made (or an empty one). Anything else may be a real
# checkout the caller pointed at by mistake.
if [[ -n "$(ls -A "$OUT" 2>/dev/null)" ]]; then
  if [[ -f "$OUT/locales/$LOCALE/$APP.tsv" ]]; then
    echo "==> replacing the existing catalog checkout at $OUT"
    rm -rf "${OUT:?}/locales"
  else
    die "$OUT is not empty and does not look like a craft-i18n checkout; remove it or choose another -o"
  fi
fi

TMP="$(mktemp -d "${TMPDIR:-/tmp}/craft-i18n-korean.XXXXXX")"
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

echo "==> catalogs: $REPO @ $REV"
fetch_commit "$REPO" "$REV" "$TMP/src"
git -C "$TMP/src" archive --format=tar FETCH_HEAD -- locales | tar -x -C "$OUT"

CAT="$OUT/locales/$LOCALE/$APP.tsv"
[[ -f "$CAT" ]] || die "the pinned checkout has no locales/$LOCALE/$APP.tsv"

# Whether this exact (context, source) row is already there and equal/unequal, or absent.
row_state() {
  LC_ALL=C awk -F'\t' -v ctx="$2" -v src="$3" -v want="$4" '
    $1 == ctx && $2 == src { print ($3 == want ? "same" : "other"); found = 1; exit }
    END { if (!found) print "absent" }
  ' "$1"
}

# Insert at the catalog's sort position: source text casefolded, then source, then context.
# Catalogs edited by hand are not always perfectly sorted; this lands next to the neighbours
# upstream's own tools would choose.
insert_row() {
  local file="$1" ctx="$2" src="$3" tr="$4" tmp="$1.tmp"
  LC_ALL=C awk -F'\t' -v ctx="$ctx" -v src="$src" -v tr="$tr" '
    BEGIN { done = 0; low = tolower(src) }
    {
      if (!done && $0 !~ /^#/ && NF == 3 && tolower($2) > low) {
        printf "%s\t%s\t%s\n", ctx, src, tr
        done = 1
      }
      print
    }
    END { if (!done) printf "%s\t%s\t%s\n", ctx, src, tr }
  ' "$file" >"$tmp"
  mv "$tmp" "$file"
}

# Update an existing row in place (its sort position does not change for a typo fix).
replace_row() {
  local file="$1" ctx="$2" src="$3" tr="$4" tmp="$1.tmp"
  LC_ALL=C awk -F'\t' -v ctx="$ctx" -v src="$src" -v tr="$tr" '
    $1 == ctx && $2 == src { printf "%s\t%s\t%s\n", ctx, src, tr; next }
    { print }
  ' "$file" >"$tmp"
  mv "$tmp" "$file"
}

added=0
updated=0
present=0
while IFS=$'\t' read -r source translation; do
  [[ -n "$source" ]] || continue
  case "$source" in
    [[:space:]]* | *[[:space:]]) die "a row source has leading or trailing whitespace: $source" ;;
  esac
  case "$translation" in
    [[:space:]]* | *[[:space:]]) die "a row translation has leading or trailing whitespace: $source" ;;
  esac
  case "$source$translation" in
    *'{'* | *'}'*) die "a face-label row must not carry a placeholder: $source" ;;
    *'…'*) die "a face-label row must not carry an ellipsis: $source" ;;
  esac
  state="$(row_state "$CAT" "" "$source" "$translation")"
  case "$state" in
    same) present=$((present + 1)) ;;
    other)
      replace_row "$CAT" "" "$source" "$translation"
      updated=$((updated + 1))
      ;;
    absent)
      insert_row "$CAT" "" "$source" "$translation"
      added=$((added + 1))
      ;;
    *) die "could not read $CAT for '$source'" ;;
  esac
done <<'EOF'
Hakgyoansim Namu	학교안심 나무
Hakgyoansim Sangjang	학교안심 상장
Hakgyoansim Chulseokbu	학교안심 출석부
NanumGothic	나눔고딕
EOF

# ---- self-check --------------------------------------------------------------------------------
[[ $((added + updated + present)) -eq "$ROWS" ]] || die "expected $ROWS rows handled, got $((added + updated + present))"

# Every non-comment row must have exactly three TAB-separated columns ...
LC_ALL=C awk -F'\t' '!/^#/ && NF { if (NF != 3) { printf "%s: line %d has %d columns\n", FILENAME, NR, NF; bad = 1 } } END { exit bad }' "$CAT" ||
  die "$CAT is not a three-column catalog"

# ... and no duplicate (context, source) key.
DUPES="$(LC_ALL=C awk -F'\t' '!/^#/ && NF { k = $1 SUBSEP $2; if (seen[k]++) { print NR ": " $2; bad = 1 } } END { exit bad }' "$CAT" 2>/dev/null || true)"
[[ -z "$DUPES" ]] || die "$CAT has duplicate keys: $DUPES"

# Every row lands with the exact translation, and no control character slipped in.
while IFS=$'\t' read -r source translation; do
  [[ -n "$source" ]] || continue
  [[ "$(row_state "$CAT" "" "$source" "$translation")" == "same" ]] || die "'$source' did not land in $CAT"
  if LC_ALL=C grep -Fq "$(printf '\t%s\t' "$source")" "$CAT"; then :; else die "'$source' is not in $CAT"; fi
done <<'EOF'
Hakgyoansim Namu	학교안심 나무
Hakgyoansim Sangjang	학교안심 상장
Hakgyoansim Chulseokbu	학교안심 출석부
NanumGothic	나눔고딕
EOF

LINES="$(wc -l <"$CAT" | tr -d ' ')"
if grep -Eq "^	Korean	" "$OUT/locales/$LOCALE/common.tsv" 2>/dev/null; then
  LANGUAGE_ROW="Korean -> 한국어 is upstream (common.tsv), left alone"
else
  LANGUAGE_ROW="warning: upstream common.tsv has no Korean language row"
fi

echo "==> $OUT"
echo "rows added: $added"
echo "rows updated: $updated"
echo "rows already present: $present"
echo "catalog: locales/$LOCALE/$APP.tsv ($LINES lines)"
echo "duplicate keys: none"
echo "column check: every row has 3 TAB-separated columns"
echo "language row: $LANGUAGE_ROW"
echo "verified: 4 Korean face-label rows resolve with the exact translations"
