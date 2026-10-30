#!/usr/bin/env bash
# make-dmg-local.sh — build the drag-to-Applications DMG for a labelled kstyles macOS build.
#
# The tar.gz releases carry the same ad-hoc-signed PdfCraft.app the DMG would, but hdiutil only
# runs on macOS, so this ports the CI recipe (kstyles-macos-dmg.yml) to the Mac itself:
#   1. unpack the release tar.gz
#   2. give the bundle a distinct CFBundleVersion (0.15.0.kstylesNN): upstream pins
#      CFBundleVersion to the bare short version, so consecutive kstyles builds are the "same
#      version" to Finder/LaunchServices — that is what makes upgrading behave oddly
#   3. re-sign ad-hoc inside-out (hardened flag, empty entitlements — same as upstream)
#   4. hdiutil create UDZO with the repo's background.tiff + dmg-layout.DS_Store, /Applications link
#   5. codesign --verify --strict inside the mounted image (the #390 detritus gate), detach
#   6. print the DMG + its sha256 (attach to the release: gh release upload TAG *.dmg --clobber,
#      or rsync it back to the build box)
#
# Usage (on the Mac):  bash make-dmg-local.sh <pdfcraft-...-macos-aarch64.tar.gz>
#                     bash make-dmg-local.sh --url https://github.com/homeass/pdfcraft/releases/download/v0.15.0-kstyles12/pdfcraft-0.15.0-kstyles12-macos-aarch64.tar.gz
# Needs: macOS, Xcode command-line tools (hdiutil, codesign, plutil), curl. Ad-hoc signed:
# first launch still needs right-click ▸ Open (see packaging/macos/kmacos1-notes.md).
set -euo pipefail

RAW=https://raw.githubusercontent.com/homeass/pdfcraft/feat/16-windows/packaging/macos/dmg
TARGZ=${1:-}
if [ "$TARGZ" = "--url" ]; then
  TARGZ=$(basename "${2:?missing url}")
  echo "==> downloading ${2}"
  curl -fL -o "$TARGZ" "$2"
fi
[ -n "$TARGZ" ] && [ -f "$TARGZ" ] || { echo "usage: $0 <file.tar.gz> | --url <tar.gz url>" >&2; exit 2; }

step() { echo; echo "==> $*"; }
die() { echo "FAIL: $*" >&2; exit 1; }

command -v hdiutil >/dev/null || die "run this on macOS"
command -v codesign >/dev/null || die "Xcode command-line tools missing (xcode-select --install)"

WORK=$(mktemp -d /tmp/pdfcraft-dmg.XXXXXX)
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"

step "unpack"
tar xzf "$OLDPWD/$TARGZ" 2>/dev/null || tar xzf "$TARGZ"
test -x PdfCraft.app/Contents/MacOS/PdfCraft || die "no PdfCraft.app in $TARGZ"

VFULL=$(plutil -extract PdfCraftVersion raw PdfCraft.app/Contents/Info.plist)
[ -n "$VFULL" ] || die "Info.plist carries no PdfCraftVersion (is this a kstyles build?)"
SHORT=${VFULL%%-*}; LABEL=${VFULL#*-}
BUNDLE_VERSION="$SHORT.$LABEL"
step "version the bundle: CFBundleVersion -> $BUNDLE_VERSION (was $(plutil -extract CFBundleVersion raw PdfCraft.app/Contents/Info.plist))"
plutil -replace CFBundleVersion -string "$BUNDLE_VERSION" PdfCraft.app/Contents/Info.plist

step "ad-hoc re-sign (inside-out, hardened flag, empty entitlements)"
ENT=$WORK/empty-entitlements.plist
cat >"$ENT" <<'XML'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict></dict></plist>
XML
codesign --force --sign - --timestamp=none --options runtime --entitlements "$ENT" PdfCraft.app/Contents/MacOS/PdfCraft
codesign --force --sign - --timestamp=none --options runtime --entitlements "$ENT" PdfCraft.app
codesign --verify --strict --deep --verbose=2 PdfCraft.app

step "fetch the DMG chrome (background + icon layout)"
curl -fL -o background.tiff "$RAW/background.tiff"
curl -fL -o dmg-layout.DS_Store "$RAW/dmg-layout.DS_Store"

step "hdiutil create (UDZO, HFS+, volname PdfCraft)"
STAGE=$WORK/stage; mkdir -p "$STAGE/.background"
ditto PdfCraft.app "$STAGE/PdfCraft.app"
ln -s /Applications "$STAGE/Applications"
cp background.tiff "$STAGE/.background/background.tiff"
cp dmg-layout.DS_Store "$STAGE/.DS_Store"
DMG=pdfcraft-$VFULL-macos-aarch64.dmg
for attempt in 1 2 3 4 5; do
  if hdiutil create -srcfolder "$STAGE" -volname PdfCraft -fs HFS+ -format UDZO -imagekey zlib-level=9 -ov "$DMG"; then break; fi
  [ "$attempt" = 5 ] && die "hdiutil create failed 5 times"
  echo "retry $attempt"; sleep $((attempt * 10))
done
codesign --force --sign - --timestamp=none "$DMG"
codesign --verify --strict --verbose=2 "$DMG"

step "verify the app inside the mounted image (the #390 detritus gate)"
MOUNT=$WORK/mount; mkdir -p "$MOUNT"
hdiutil attach -nobrowse -readonly -mountpoint "$MOUNT" "$DMG" >/dev/null
codesign --verify --strict --deep --verbose=2 "$MOUNT/PdfCraft.app"
hdiutil detach "$MOUNT" >/dev/null

step "done"
OUT=${OLDPWD:-.}/$DMG
mv "$DMG" "$OUT"
ls -l "$OUT"
echo "sha256: $(shasum -a 256 "$OUT" | cut -d' ' -f1)"
echo "attach it:  gh release upload ${SHORT}-${LABEL} $(basename "$OUT") --clobber  (needs gh auth)"
echo "     or rsync $(basename "$OUT") back to the build box for attaching there"
