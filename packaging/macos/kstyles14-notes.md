# kstyles14 — the Korean chain rebased onto upstream 0.17.0 (macOS arm64 built on the Mac)

Unofficial test build by homeass, not an upstream ArtCraft release. Continues the Korean chain
(`v0.16.1-kstyles13`) on top of `origin/main` at **0.17.0** — branch `feat/17-kstyles`, tag
`v0.17.0-kstyles14`, `--version` reports `0.17.0 (kstyles14)`.

## Replay

36 commits replayed with `git rebase --onto origin/main v0.16.1` from tag `v0.16.1-kstyles13`
(no squash). Upstream drifted 214 commits / 16.5k insertions between the two versions, so:

- 34 `crates/ui-egui/src/i18n/*.tsv` conflicts: keep upstream's rows, re-add only the port's
  Korean face-name rows. 0.17.0 added a hard test
  (`image_paste_and_home_notifications_are_translated`) asserting every non-English catalog carries
  `Search…`, `Image pasted`, `No image on clipboard to paste`, `Pinned {tool} to Home`,
  `Unpinned {tool} from Home` — those must survive the merge or the suite fails.
- `ATTRIBUTION.md` is generated: `cargo xtask assets --write`, never hand-merged.
- 0.17.0 introduced `LICENSES.toml` (hashed source documents) *after* the port's Tinos commit had
  reworded `NOTICE`, so the recorded hash was upstream's text and `cargo xtask license-check` failed.
  The hash was re-audited to the port's `NOTICE` (upstream's kept in the comment above the entry).
  The gate has no `--write` mode, so this is a deliberate hand edit.
- The OCR surface needed no adaptation: `crates/ocr/src/lib.rs`, `crates/engine/src/ocr.rs`,
  `crates/ui-egui/src/workflows/ocr_ui.rs`, `crates/automation/src/a11y/ocr.rs` and
  `crates/fonts/build.rs` are blob-identical between v0.16.1 and v0.17.0. Real work was in
  `edit_text_ui.rs`/`layout.rs`, `format_text_ui.rs`, `edit/src/lib.rs`, `text/fallback.rs`
  (upstream made `strip_subset_tag` pub while the port reshaped `Script`) and `engine/src/edit/run.rs`
  (upstream moved `EditPageImage` next to the port's `AddOcrText` arm).
- `cargo xtask license-check` also keeps the chain's Korean ledger alive: `korean-recognition.onnx`
  and `korean-dict.txt` as `[[fetched]] kind = "model"` (Apache-2.0) plus the fixture as a repo asset,
  so `package.sh` ships them beside the Latin OCR models.

## Build recipe (macOS 27, Apple silicon, 2026-11-02)

```
export PATH=$HOME/.cargo/bin:$PATH          # Homebrew rustc 1.93 is below rust-version 1.95
export CRAFT_FONTS_DIR=<work>/craft-fonts-mac CRAFT_FONTS_REQUIRED=1
export PDFCRAFT_VERSION=0.17.0-kstyles14    # artifact names + Info.plist PdfCraftVersion
export PDFCRAFT_BUILD_LABEL=kstyles14       # --version / About; env.sh does NOT fold it into VERSION
packaging/macos/package.sh --arch aarch64
bash packaging/macos/make-dmg-local.sh dist/release/pdfcraft-0.17.0-kstyles14-macos-aarch64.tar.gz
```

## This machine: `hdiutil attach` and external volumes

`package.sh`'s in-image verification gate mounts the DMG at `$CARGO_TARGET_DIR/macos-package/dmg-check`.
When the workspace lives on an external volume, `hdiutil attach -mountpoint <path on that volume>`
fails here with `권한 거부됨` (EPERM) even though `hdiutil create` and signing succeed — so
`package.sh` stops after the DMG is built (the CLI zip step never runs). Mounting the same image at a
`/tmp` mountpoint works. `make-dmg-local.sh` already stages under `mktemp -d /tmp/…`, so run the final
DMG through it: it re-signs inside-out, sets a distinct `CFBundleVersion` (`0.17.0.kstyles14`) and does
its own mount + `codesign --verify --strict` check. Do the same for any manual check of `package.sh`'s
output: `hdiutil attach -nobrowse -readonly -mountpoint /tmp/… <dmg>`.

## Verification

- `cargo check --workspace --all-targets` · `cargo fmt --check` · `cargo clippy -D warnings`
  (ocr/edit/engine/ui-egui/render) · `cargo check -p pdfcraft-ocr --target aarch64-apple-darwin --locked`
  (`package.sh` builds `--locked`, so the lock must carry the port's `objc2*`/`png` deps)
- Korean suites: render 3 + edit 13 (`-- korean`), ocr 17, `ocr_korean_layer` 2, engine e2e 7
  incl. `korean_scan_becomes_searchable_through_vision` (real Vision call), automation 2, i18n 64
- `cargo xtask assets` (290 repo assets, 24 fetched at build time) · `license-check` · `parity`
- 22 craft faces embedded from `craft-fonts-mac` (NanumGothic default, Namu/Sangjang/Chulseokbu,
  Tinos ×4 …) — the `Hans` manifest face is required since 0.15, so the merged font input must carry
  `fonts/simplified-chinese-manifest.txt`
- Bundled app: `codesign --verify --strict --deep`, `PdfCraft --version` → `0.17.0 (kstyles14)`,
  `Contents/Resources/models` holds the Latin detection/recognition ONNX + `korean-recognition.onnx`
  + `korean-dict.txt` with their licences
- Ad-hoc signed, not notarized: first launch on another Mac needs
  `xattr -d com.apple.quarantine /Applications/PdfCraft.app` or right-click ▸ Open
- Known gap: `crates/ocr/src/paddle.rs` (Linux/Windows OCR) is cfg-gated off on macOS, so this build
  neither type-checks nor tests it — the fork's Linux CI covers it.

## Native Linux and Windows builds

The merged font input is reproducible from pinned public refs:
`packaging/craft-fonts-korean-merge.sh -o craft-fonts-korean`. The two native jobs build, verify
(PaddleOCR tests plus a headless Korean roundtrip) and attach their artifacts to the tag:

```
gh workflow run kstyles-linux-deb.yml   --ref feat/17-kstyles -f tag=v0.17.0-kstyles14
gh workflow run kstyles-windows-zip.yml --ref feat/17-kstyles -f tag=v0.17.0-kstyles14
```

GitHub registers `workflow_dispatch` only for workflows that exist on the default branch: the two
files must be on `main` once (workflow files only; they check out `inputs.ref` themselves).

Both fail if Korean does not come back from the roundtrip; Windows still ships the portable zip
when WiX/MSI fails, marking the MSI skipped in the run summary.

## CI 결과 (2026-11-02): 세 플랫폼.green on the same tag

`gh workflow run kstyles-linux-deb.yml` / `kstyles-windows-zip.yml` both dispatched from `main`
with `-f tag=v0.17.0-kstyles14 -f ref=feat/17-kstyles`, and both are green. Native evidence:

- Ubuntu 24.04 x86_64: `cargo test --release -p pdfcraft-ocr` → 18 passed (`paddle.rs`, which a macOS
  build never compiles), deb + rpm + FHS tar + CLI tar built, `sudo dpkg -i` then
  `pdfcraft --version` == `pdfcraft 0.17.0 (kstyles14)`, Korean roundtrip twice (built CLI and installed
  CLI) → 32 Hangul characters re-extracted.
- Windows Server x64: same 18 paddle tests, MSI via WiX 5 (unsigned: the fork has no signing secrets) +
  portable zip; roundtrip from the extracted zip with `models\` beside `pdfcraft-cli.exe` → 16 words,
  32 Hangul characters.
- `packaging/craft-fonts-korean-merge.sh` output equals the hand-made merged directory byte for byte
  (checked on macOS), so all three platforms embed the same 22 faces.

Two dispatch-workflow bugs were found and fixed on the way, both silent-until-CI: `$GITHUB_ENV` cannot
carry a bare bin directory (must be `$GITHUB_PATH`), and `pdfcraft-*-linux-x86_64.tar.gz` also matches the
CLI tarball, so the "exactly one FHS tar.gz" assertion failed at the upload step. Note that a re-run of a
workflow_dispatch job uses the YAML from the *original* commit — fix the file on the default branch and
dispatch a fresh run, `gh run rerun` will not pick the change up.

Known consequences of building Linux on ubuntu-24.04: glibc 2.39 floor (despite nfpm's `libc6 >= 2.35`),
AppImage intentionally not built (headless run unproven on the runner), and the release now carries 12
assets — the Linux CLI tarball is new versus kstyles13.
