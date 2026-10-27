# kmacos1 — Korean faces for macOS (arm64) test build

Unofficial test build by homeass, not an upstream ArtCraft release.
Ports the Linux deb Korean-face work (tag `v0.14.0-kstyles9`, source commit
`6fefb2d9`) to macOS Apple silicon. No source changes: upstream
`packaging/macos/package.sh` already embeds `CRAFT_FONTS_DIR` and ships the OFL
notices, so the port is a build recipe.

## Recipe (verified on macOS 27, Apple silicon, 2026-10-27)

- Toolchain: rustup stable 1.99.1 (workspace requires rustc >= 1.95; Homebrew's
  1.93 is too old), Xcode clang for linking. Host: macOS 27, Apple silicon.
- Font input: one merged `CRAFT_FONTS_DIR` checkout —
  storytold/craft-fonts @ `b076417b` (adds NanumGothic + all shared faces,
  incl. the Simplified-Chinese manifest), plus `homeass/craft-fonts`
  `feat/hakgyoansim-namu` (Namu R/L, Sangjang R, Chulseokbu L/B: font dirs and
  their `Kore` manifest lines) and `feat/tinos-ui-serif` (Tinos: font dir and
  its four manifest lines). `git archive` the font dirs, append the manifest
  lines, then `CRAFT_FONTS_REQUIRED=1 cargo check -p pdfcraft-fonts` and confirm
  the OUT_DIR `craft_fonts.rs` lists 26 faces.
- Build: `PDFCRAFT_BUILD_LABEL=kmacos1 CRAFT_FONTS_DIR=<merged> CRAFT_FONTS_REQUIRED=1
  packaging/macos/package.sh --arch aarch64`
- Signing: ad-hoc (`-`), not notarized — first launch needs
  `xattr -d com.apple.quarantine /Applications/PdfCraft.app` or right-click ▸ Open.

## Evidence

- `cargo test --release -p pdfcraft-render -p pdfcraft-edit --lib -- korean`:
  13 passed, 0 failed (Sangjang picked for serif, Chulseokbu Light/Bold for
  Courier/bold, NanumGothic default, added Korean text round-trips).
- Built DMG: `codesign --verify --strict` passes on the mounted app;
  `pdfcraft-cli --version` reports `0.14.0 (kmacos1)`.
- Artifacts: `pdfcraft-0.14.0-macos-aarch64.dmg` (65 MB),
  `pdfcraft-cli-0.14.0-macos-aarch64.zip` (41 MB), bundled in
  `pdfcraft-0.14.0-kmacos1-macos-aarch64.tar.gz` with SHA256SUMS.txt and
  README-kmacos1.txt. Released as tag `v0.14.0-kmacos1`.
