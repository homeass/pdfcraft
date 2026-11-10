# kstyles 체인 재이식 후처리 프롬프트 v1 (그대로 붙여넣기 — 다음 업스트림용)

한글 체인(`feat/*-kstyles*`) 브랜치를 다음 업스트림 릴리스 위로 rebase한 **직후** 붙여넣는다.
`kstyles-next-upgrade.md`(§0–§2 재이식)가 끝난 지점에서 시작하며, §3 조용한 손실 증명·§4 생성파일·
§5 게이트를 통과시키는 데 실제로 걸렸던 함정과 수정법을 순서대로 적었다.
`v0.20.0`(kstyles19, 포트 63 commits + 수정 2 commits)에서 실측으로 증명된 내용만 담았다.

## 0) 전제 확인 (게이트 전에 한 번)

```bash
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin"  # Homebrew rustc는 rust-version 미달
rustc --version; cargo --version
bash packaging/craft-fonts-korean-merge.sh -o /tmp/craft-fonts-korean   # 22 faces 확인
bash packaging/craft-i18n-korean-merge.sh -o /tmp/craft-i18n-korean     # 4 rows 확인
export CRAFT_FONTS_DIR=/tmp/craft-fonts-korean CRAFT_FONTS_REQUIRED=1
export CRAFT_I18N_DIR=/tmp/craft-i18n-korean
cargo xtask models   # assets/models (git-ignored). UI OCR 테스트가 건너뛰지 않고 타려면 필수
```

`cargo xtask models` 없이 `cargo test -p pdfcraft-ui-egui --test it` 를 돌리면 OCR 테스트가
`can_undo() == None` 으로 실패한다. `ocr::available()` 이 tesseract 때문에 `true` 여도
ocrs 모델이 없으면 인식이 돌지 않으므로, 실패는 환경 문제이지 포트 코드 문제가 아니다.
모델을 내려받은 뒤 해당 테스트만 다시 돌려 먼저 가려낼 것.

## 1) fmt 게이트가 깨지면

- 증상: `cargo fmt --all -- --check` 가 포트 손댄 파일에서만 diff를 뱉는다.
- 수정: `cargo fmt --all` 실행 → `git diff --stat` 으로 범위가 포트 파일뿐인지 확인 →
  커밋에 포함. (이번: `draw.rs` 15줄, `shaped_text.rs` 4줄, `ocr/src/lib.rs` 8줄)
- 검증: `cargo fmt --all -- --check` → exit 0.

## 2) Cargo.lock 충돌이 나면

- 손으로 버전을 고르지 않는다. 양쪽 `rten`·`serde_json`·`sha2` 요구를 합집합으로 유지한다.
  (이번: `rten 0.24.0`+`0.26.0`, `serde_json`/`sha2` 복원)
- 검증: `cargo check -p pdfcraft-ocr --target aarch64-apple-darwin --locked` → exit 0.

## 3) 업스트림이 포트 패치 지점을 리팩터했으면 (충돌 3종 정석)

원칙: **업스트림의 새 구조를 유지하고, 포트 분기만 그 위에 재이식**한다. 옛날 본문을
통째로 살리지 말 것.

- `engine/src/ocr.rs` 의 `run()` → `OcrSteps`/`OcrShot` 구조가 오면: steps 기반
  `run()` 은 그대로 두고, 포트 분기는 `prepare()` 안 ko 조기반환 +
  `read()` → `read_korean()` dispatch 로 얹는다. ko가 아니면 기존 경로와 바이트 단위로
  같아야 한다 (이번 43줄 diff가 그 전부여야 함).
- `draw.rs` 가 단일 `ShapedFonts` 로 바뀌었으면: ko/rest 를 분리해 `PCKo`/`PCAr` 두
  개로 만든다 (혼합-스크립트 한 줄이 한쪽에 몰리면 안 됨).
- `ShapedFonts::new`/`type3_font` 시그니처가 바뀌었으면: `korean_face: Option<&CraftFont>`
  인자를 따라가고, **모든 호출부**를 갱신한다 (`appearance_lines`, `add_type3_font`,
  테스트 포함 — 빠진 한 곳이 컴파일 에러로 바로 드러남). Korean chunk 명명은
  `Family-Style` 형식을 유지한다 (이름→페이스 역조회가 끊기면 재타이핑이 조용히 깨짐).

## 4) OCR 모델: 먼저 깔고 테스트를 가려낼 것

- `cargo xtask models` 를 먼저 실행한다 (위 §0).
- 그래도 실패하는 것만 포트 문제로 본다. (이번: 모델 설치 후 it ocr 2종 통과.)

## 5) editor가 첫 프레임에 안 뜨면: `install_source_fonts` 의 `choice_only` 필터를 의심하라 (핵심 회귀 사례)

- 증상: `a_paragraph_editor_opens_at_full_opacity` 가 `frame 0` 에서 실패한다
  (`left: None`, `right: Some(#14_73_E6_FF)`).
- 판별: fonts on/off × i18n on/off 매트릭스로 입력을 가른다. 이번엔 fonts만 있을 때
  실패했고(i18n 단독은 통과), 테스트 자체는 업스트림 것이다(포트 diff에 없음).
- 원인: 포트가 `install_source_fonts` 에서 upstream의 `choice_only` 필터를 떨어뜨렸다.
  upstream 주석이 경고한 그대로다 — 쓰지 않는 choice(번들 Thai 대체 선택지)를 설치하면
  font atlas 재구축 때문에 **표준 폰트 editor조차 한 프레임을 기다린다**. CRAFT_FONTS 가
  있을 때만 Thai face가 존재하므로, fonts 있을 때만 실패한다.
- 수정: 필터와 주석을 원복한다 (Korean fallback은 `preview_family` 경로라 영향 없음):
  `doc.text_editor_fonts(page).into_iter().filter(|font| !font.choice_only)`.
- 검증: `cargo test -p pdfcraft-ui-egui --test it a_paragraph_editor_opens_at_full_opacity`
  (fonts+i18n) → pass. 그 뒤 `it editing` 전체 → 304 passed, 0 failed.

## 6) upstream OCR API가 바뀌었으면 테스트 호출부를 함께 옮긴다

- `reads_rendered_text` 류: 구 `Models::find` + `Ocr::load` + `ocr.recognize(&px, w, h)` →
  신 `OcrsRecognizer::find` + `OcrImage::new` + `rec.recognize(&image, &RecognizeOptions { language: "en".into() })`.
- 규칙: upstream이 바꾼 API는 포트가 끝까지 따라간다 (구 API에 어댑터 두지 않기).

## 7) 생성파일 3종 (손대지 말고 재생·재감사만)

- `ATTRIBUTION.md` 는 `cargo xtask assets --write` 로만. `cargo xtask assets` 가
  `ok (...)` 이어야 한다.
- `NOTICE` ↔ `LICENSES.toml` 의 NOTICE 해시는 한 쌍으로 이동 (`--write` 없음).
  upstream이 해시 대상 문서를 바꿨으면 1줄만 재감사하고 이전 해시는 주석으로 남긴다.
- `parity/acrobat-features.toml` 은 HEAD 기반을 유지하고 한글 행만 추가한 뒤,
  `cargo xtask parity` 가 깨지지 않는지(이번: 857 features) 확인한다.

## 8) 게이트 매트릭스 (전부 exit 0이어야 통과)

```bash
cargo check --workspace --all-targets            # fonts 없이도 한 번
CRAFT_FONTS_DIR=... CRAFT_FONTS_REQUIRED=1 CRAFT_I18N_DIR=... cargo check --workspace --all-targets
cargo fmt --all -- --check
cargo clippy -p pdfcraft-ui-egui -p pdfcraft-edit -p pdfcraft-render -p pdfcraft-automation --all-targets -- -D warnings
cargo xtask assets && cargo xtask license-check && cargo xtask parity
actionlint .github/workflows/kstyles-*.yml && bash -n packaging/craft-*.sh packaging/macos/make-dmg-local.sh
cargo test --release -p pdfcraft-render -p pdfcraft-edit --lib -- korean
cargo test --release -p pdfcraft-ocr --lib
cargo test -p pdfcraft-ui-egui --test it editing     # 전수 (이번: 304 passed)
cargo test -p pdfcraft-ui-egui --test it korean
cargo test -p pdfcraft-ui-egui --test it fonts
cargo test -p pdfcraft-ui-egui --test it ime_text
cargo test -p pdfcraft-ui-egui --lib i18n
cargo test -p pdfcraft-automation retyping
cargo test -p pdfcraft --bin pdfcraft flatpak
```

- 실패가 나면: ① 모델·폰트·i18n 입력 문제인지 먼저 가린다(§0, §4),
  ② 그래도 실패하면 포트가 건드린 파일인지 `git diff <base>..HEAD -- <파일>` 로 좁힌다,
  ③ 포트 원인이면 최소 수정 후 같은 게이트로 증명한다 (이번 choice_only 건이 모범 사례).
- `actionlint` 가 없으면 `brew install actionlint` (CI 이미지엔 없을 수 있음).

## 알려진 함정 추가 (kstyles14·15·16·17 + 이번 실측)

| 함정 | 증상 | 대비 |
|---|---|---|
| rebase 뒤 fmt 방치 | `fmt --check` 가 포트 파일 3곳에서만 실패 | §1: `cargo fmt --all` 후 범위 확인·커밋 |
| `choice_only` 필터 탈락 | editor가 frame 0에 안 뜸 (fonts 있을 때만) | §5: 필터+주석 원복, it editing 전수 |
| OCR 모델 미설치 | it ocr 2종 `can_undo() == None` | §0·§4: `cargo xtask models` 먼저 |
| `ShapedFonts::new` 호출부 누락 | 컴파일 에러 | §3: 호출부 전수 갱신 (`None` 전달) |
