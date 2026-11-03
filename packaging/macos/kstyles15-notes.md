# kstyles15 — the Korean chain on upstream 0.18.0 (all four artifacts from CI)

Unofficial test build by homeass, not an upstream ArtCraft release. Replays the chain
(`v0.17.0-kstyles14`, 44 port commits) onto **0.18.0** as branch `feat/18-kstyles`, tag
`v0.18.0-kstyles15`, `--version` `0.18.0 (kstyles15)`. Test builds: ad-hoc / unsigned on purpose —
no Apple or Azure signing secrets are required for this channel.

## Replay

`git rebase --onto refs/remotes/origin/main bee7ba43 origin/feat/17-kstyles` — 43 commits, no squash.
Upstream moved 0.17.0→0.18.0 by 142 commits, but the chain and upstream only share **10 files**, and
exactly **one** of them conflicted:

- `crates/ui-egui/src/panels/format_text_ui.rs` — 0.18.0 refactored the panel into
  `outlined()/boxed()/centred()/preset_arrow()` helpers, and the port's Tinos preview edit lived inside
  the same `show_ui` closure. Resolution: take 0.18.0's file and point the single
  `for f in [FontFamily::Helvetica, Times, Courier]` loop inside `outlined(...)` at
  `edit_text_ui::preview_family(f, false, false)` + `edit_text_ui::family_label_job(f, look.bold, body)`.
  **Taking upstream and stopping there silently drops the preview** (Times then looks like Helvetica)
  and no test asserts it — check with
  `git diff v0.18.0 -- crates/ui-egui/src/panels/format_text_ui.rs`, which must show only that loop.

Everything else merged mechanically, and a set of files was verified to be **byte-identical between
0.17.0 and 0.18.0**, which is why this replay needed no API adaptation at all: `xtask/`, `packaging/`,
`apps/`, `crates/platform/`, `crates/fonts/`, `LICENSES.toml`, `NOTICE`, `ATTRIBUTION.toml`,
`crates/ui-egui/src/i18n/tests.rs`, and the four OCR integration points (`crates/ocr/src/lib.rs`,
`crates/engine/src/ocr.rs`, `crates/ui-egui/src/workflows/ocr_ui.rs`, `crates/automation/src/a11y/ocr.rs`).

## Build — one dispatch per platform, nothing hand-run

```
gh workflow run kstyles-macos.yml   --ref main -f tag=v0.18.0-kstyles15 -f ref=feat/18-kstyles   # dmg, cli zip, app tar.gz, SHA256SUMS + Vision roundtrip
gh workflow run kstyles-linux-deb.yml   --ref main -f tag=v0.18.0-kstyles15 -f ref=feat/18-kstyles # deb, rpm, tar, cli tar + dpkg install + Paddle roundtrip
gh workflow run kstyles-windows-zip.yml --ref main -f tag=v0.18.0-kstyles15 -f ref=feat/18-kstyles # msi, portable zip + Paddle roundtrip
```

`tag`, `ref`, `version` and `label` are inputs, and each job derives `version`/`label` from the tag
(`v<version>-<label>`) — so kstyles16 is another dispatch, not an edit. `workflow_dispatch` requires the
workflow files to exist on the **default branch**, so they are registered on `main` (they build
`inputs.ref`, nothing else lives there). Re-running a finished run (`gh run rerun`) uses the YAML from
the *original* commit: after editing a workflow, dispatch a fresh run.

Platform env facts (from `packaging/env.sh`, which does **not** fold the label into `$VERSION`):

- macOS needs both `PDFCRAFT_VERSION=0.18.0-kstyles15` (file names + `PdfCraftVersion`) and
  `PDFCRAFT_BUILD_LABEL=kstyles15` (`--version`/About). Omitting the first makes `make-dmg-local.sh`
  compute `CFBundleVersion=0.18.0.0.18.0` and strip the label from the DMG name.
- Linux needs the label only — `package.sh` writes `LABEL_SUFFIX` into the file names itself
  (setting `PDFCRAFT_VERSION` there would double the label).
- Windows `package.ps1` has no suffix logic, so the label must ride inside `PDFCRAFT_VERSION`.
- The merged Korean font input comes from `packaging/craft-fonts-korean-merge.sh`
  (`storytold/craft-fonts@b076417b` + school-safe + Tinos refs → 22 faces / 6 Kore / 4 Tinos / 1 Hans).
  0.18.0 kept the same `craft-fonts-ref` pin, so the script needed no change.

## Verification baked into the jobs

Every job is its own gate: `cargo test --release -p pdfcraft-ocr` (18 tests — `paddle.rs` is not even
compiled by a macOS build), then a headless Korean roundtrip that must return Hangul
(`ocr_recognize {language:"ko"}` → save → `pdfcraft-cli text`, ≥20 Hangul chars), an exact
`pdfcraft 0.18.0 (kstyles15)` version assertion, and on Linux an extra roundtrip **after `dpkg -i`**
against `/usr/bin` and `/usr/share/pdfcraft/models`. macOS runs the same roundtrip through Vision.

Local gates before dispatching: `cargo check --workspace --all-targets`, `cargo check -p pdfcraft-ocr
--target aarch64-apple-darwin --locked` (all three packaging scripts build `--locked`, so the port's
`objc2*`/`ocrs-cjk`/`png` deps must be in `Cargo.lock`), the Korean suites (edit 13 / render 3),
i18n 64 including the hard notification test, `cargo fmt --check`, clippy `-D warnings`, and
`cargo xtask assets --write` + `license-check` + `parity` (`assets` is not part of `cargo xtask ci`).

## Install over an existing kstyles build

The DMG's `CFBundleVersion` is `0.18.0.kstyles15`, so Finder/LaunchServices treat it as a distinct app
version. Copy with `rm -rf /Applications/PdfCraft.app && ditto <mounted>/PdfCraft.app /Applications/` —
a plain `ditto` **merges**, and last time leftover `Contents/Resources/models/*.rten` files from an older
build survived and broke `codesign --verify --strict`.

다음 업그레이드의 절차·함정 재사용 프롬프트는 `kstyles-next-upgrade.md`에 있다.
