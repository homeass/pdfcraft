# kstyles 체인의 다음 버전 업그레이드 (재사용 프롬프트)

비공식 한글 체인 브랜치 `feat/<N>-kstyles`과 `v<버전>-kstylesNN` 릴리스를 새 업스트림 버전으로
올릴 때 쓰는 절차서 겸 프롬프트. 아래 블록을 그대로 붙여넣고 `vX.Y`/`kstylesNN`만 바꾼다.
근거·함정은 kstyles14(0.17.0)·15(0.18.0)·16(편집기 교정)에서 실측한 것.

한글 편집기의 표시/저장 서체 결함은 여기서 가장 자주 놓치는 항목이라 §7의 이중 검증을 게이트로 두었다
(관련 기록: `kstyles14-notes.md`, `kstyles15-notes.md`).

---

붙여넣기 전에 현재값 확인: `git fetch --all --tags && git tag | grep -E '^v0\.' | sort -V | tail -3`
`git tag | grep kstyles | sort -V | tail -1` `git log -1 --format=%h refs/remotes/origin/main`

---

```
- `github/homeass/pdfcraft`(한글 폰트·한글 OCR 체인 포크)를 버전 업그레이드 (0.18.2 → vX.Y)
- 한글 체인(태그 `v0.18.2-kstyles17`, 브랜치 `feat/18.2-kstyles17`)을 새 업스트림 위로 재이식하고
  릴리스 **vX.Y-kstylesNN** 태그로 배포(macOS arm64 · Ubuntu x86_64 · Windows x64, 전부 네이티브 CI)
- 세 플랫폼 모두 GitHub Actions 네이티브 러너에서 빌드·검증하고 같은 태그에 부착(로컬 수동 산출물 없음)
- Mac Studio의 기존 `/Applications/PdfCraft.app`에 덮어쓰기 설치하고 실행·스모크 확인
- 테스트 빌드이므로 서명·공증은 의도적 생략(ad-hoc/무서명)
- 세션 내용을 md로 워크스페이스(`/Volumes/ExternalSSD/Code/work`)에 저장하고, 이 프롬프트 섹션을 마지막에 갱신해 유지

## 진행 순서
0) **베이스가 무엇인지 먼저 확정**: `git merge-base --is-ancestor <태그> refs/remotes/upstream/main`가 NO면
   그 릴리스 태그는 trunk에 없다(0.18.2가 정확히 그랬다 — trunk는 그 뒤 168개 PR). 이때는 오너에게
   "릴리스 태그 vs trunk"를 물어 정하고, 포크 `main`은 trunk 미러로 그대로 두어도 된다
   (워크플로가 이미 main에 등록돼 있으면 dispatch에 지장 없음).
   커밋 메시지로 콘텐츠를 추론하지 말 것: `git show --stat <태그>` — "Release 0.18.2" 한 커밋이 186파일 +22,684/−1,874
   콘텐츠 커밋이었다.
1) 동기화: `git fetch --all --tags` → 업스트림 태그 vX.Y 확인 → 포크 `main`이 포함 안 하면
   `git worktree add --detach /tmp/wt refs/remotes/origin/main`에서 `git merge upstream/main` 후
   `git push origin HEAD:main`(저장소 안에서 worktree 경로를 상대명 `origin/main`으로 주면 디렉터리가 생기고
   push가 non-fast-forward로 오해받으니 **항상 /tmp 절대경로 + `refs/remotes/origin/main`** 리비트).
   머지 결과 확인: `git diff --name-only upstream/main HEAD`는 `.github/workflows/kstyles-*.yml`만 있어야 함.
2) 재이식: `MB=$(git merge-base origin/feat/<직전 브랜치> <새 베이스>)` →
   `git checkout -b feat/<NN> origin/feat/<직전 브랜치> && git rebase --onto <새 베이스> $MB`
   (스쿼시 금지, 업스트림 커밋 재이식 금지 — `--no-merges` 포트 커밋 수와 대조).
   겹침 사전 계산: `comm -12 <(git diff --name-only $MB..origin/feat/18-kstyles|sort) <(git diff --name-only vX.Y-1 vX.Y|sort)`.
3) 충돌 해결과 **조용한 회귀 증명**: 자동 병합돼도 기능이 사라지는 경우가 있다(kstyles15의
   `format_text_ui.rs`: 업스트림이 패널을 `outlined()/boxed()/centred()`로 리팩터하며 포트의 Tinos 프리뷰를 삼킴,
   테스트가 못 잡음). 모든 실충돌 파일에 대해 `git diff <업스트림 태그> -- <파일>`이 **포트 추가분만** 보여주는지 확인.
3b) **카탈로그/자산의 "이사"를 의심**: 체인이 편집하던 파일이 새 베이스에서 사라졌으면
   (`git log --diff-filter=D -- <path>`) 데이터가 외부로 나간 것이다. 0.18.2는 34개 i18n 카탈로그를
   `craft-i18n`(flatpak이 핀한 커밋)으로 옮겼고, 그때 우리 한글 라벨 4행은 **죽은 데이터**가 되면서도
   `tr()`이 영어로 폴백해 **테스트는 green**이었다. 대응은 폰트와 같은 합성 패턴
   (`packaging/craft-i18n-korean-merge.sh` → `CRAFT_I18N_DIR`, 업스트림 pin/flatpak 핀은 손대지 않음)이고,
   합성 없이는 스킵 이유를 명시 출력하도록 테스트를 고친다.
4) 자동 병합 파일은 스크립트로 양쪽 기능 공존을 검사(예: `korean_fallback`+`jbig2_work`, `is_korean`+`dict_in_place`,
   `"ko"`+`inherit`+`images_refused`, 한글 렌더 테스트+JBIG2 테스트, `mark_transactions`+한글 편집 테스트, parity 한글 행).
5) 생성파일 규칙: `ATTRIBUTION.md`는 `cargo xtask assets --write`로만(`assets`는 `cargo xtask ci`에 이름으로 없지만
   `license-check`가 통째로 돌려 실효 게이트다; 0.18.2에서 번역 카탈로그의 sha 면제가 폐지되어 카탈로그 항목이
   남아있으면 "listed but missing"으로 실패),
   `NOTICE`와 `LICENSES.toml`의 NOTICE 해시는 한 세트로 이동(`license-check`에 `--write` 없음 —
   업스트림이 해시 대상 문서를 바꿨을 때만 1줄 재감사하고 이전 해시는 주석에 보존), `Cargo.lock` 손편집 금지.
6) 게이트(모든 명령에 `export PATH=$HOME/.cargo/bin:$PATH`, `CRAFT_FONTS_DIR`+`CRAFT_FONTS_REQUIRED=1`):
   `cargo check --workspace --all-targets` ·
   `cargo check -p pdfcraft-ocr --target aarch64-apple-darwin --locked`(3개 패키징 스크립트 모두 `--locked` 빌드) ·
   `cargo test --release -p pdfcraft-render -p pdfcraft-edit --lib -- korean` · `cargo test --release -p pdfcraft-ocr --lib` ·
   `cargo test -p pdfcraft-ui-egui --test it editing`(전체) · `--test it korean` · `--test it ime_text` · `--lib i18n` ·
   엔진 e2e `ocr_korean`(Vision 실호출, skip 아니어야 함) · 자동화 `retyping*` ·
   `cargo fmt --all -- --check` · `cargo clippy -p pdfcraft-ui-egui -p pdfcraft-edit -p pdfcraft-render -p pdfcraft-automation --all-targets -- -D warnings` ·
   `cargo xtask assets`+`license-check`+`parity`.
7) **표시/저장 이중 검증(한글 체인 전용 필수 게이트)** — 두 층이 따로 깨진다:
   - 저장: `page_add_text font=helvetica|times|courier` → `text_lines`(기대 `NanumGothic`/`HakgyoansimSangjangR`/`HakgyoansimChulseokbuTTF`)
     → `pdfcraft-cli render` PNG 3종 해시가 서로 달라야 함.
   - 표시: 앱 실행(`--control`)→`ui.resize`→`ui.command id=edit.text`→캔버스 클릭→`ui.click label="Font"`→`ui.click label=Times|Courier|Helvetica`
     →`ui.type`→`ui.screenshot --out …`(확대 후) → 3종이 서로 다르고, 눈으로 명조/고딕/타이피라이터를 구별할 수 있어야 함.
   - 재타이핑 보존: `page_add_text font=times` → `text_edit`(font 인자 **없이**) → `text_lines`가 원본 페이스를 유지해야 함.
   - 회귀 테스트는 **반드시 수정 전에 실패하는 것**을 증명하고 커밋한다(`the_editor_preview_draws_hangul_with_the_written_korean_face`,
     `korean_face_survives_a_retype_without_a_font_request`, `retyping_added_korean_text_keeps_its_face_through_tools`,
     `retyping_an_added_korean_paragraph_in_the_editor_keeps_its_face`, `ime_text::*`가 현재 그것들).
8) 배포는 **편집이 아니라 dispatch**: 워크플로가 `tag`/`ref`/`version`/`label` 입력을 받으므로
   `gh workflow run kstyles-macos.yml|kstyles-linux-deb.yml|kstyles-windows-zip.yml --ref main -f tag=vX.Y-kstylesNN -f ref=<브랜치> [-f version=… -f label=…]`.
   `workflow_dispatch`는 기본 브랜치에 파일이 있어야 등록되니 워크플로를 바꾸면 `main`에도 반영하고,
   **`gh run rerun`은 원본 커밋의 YAML을 쓴다** — 수정 후에는 새 dispatch.
9) 커밋 이메일: author/committer 모두 `biometal.dr@gmail.com`(GH007 푸시 차단). 리베이스 후
   `git log --format='%ce' <base>..HEAD | sort -u`로 확인, 어긋났으면 `filter-branch --env-filter`로 정규화 후
   재조립(트리는 0줄 차이여야 함).
10) 설치·확인: `rm -rf /Applications/PdfCraft.app && ditto <mounted>/PdfCraft.app /Applications/`
    (plain `ditto`는 병합 → 옛 `Contents/Resources/models/*.rten` 잔여물이 `codesign --verify --strict`를 깬다),
    `--version` == `pdfcraft X.Y (kstylesNN)`, `CFBundleVersion` == `X.Y.kstylesNN`, strict 검증, 실행 확인.
    배포본 CLI로 Vision 스모크(`PDFCRAFT_MODELS=/Applications/PdfCraft.app/Contents/Resources/models`).
    백업 유지: 영어 정식판 + 직전 kstyles 본.
11) 자산 확정: 각 자산 sha256을 `gh release view --json assets`의 `digest`로 교차 확인하고
    본문·README·`SHA256SUMS.txt`(자기 자신 제외)에 반영 → 세션 md/디버그 리포트 갱신.

## 알려진 함정 (kstyles14·15·16에서 실측)
| 함정 | 증상 | 대비 |
|---|---|---|
| 프리뷰가 저장 서체를 안 따라감 | 한글이 항상 나눔고딕으로 보임(저장은 정상), 라벨만 옳은 서체로 | 에디터 폰트 스택에 스타일 헤드 한글을 넣거나 `document_korean_face_named`로 해결 |
| Type3 `/FontName`에 스타일 없음 | `Family`만 적히면 페이스 되찾기(핀)가 조용히 `None` → 재타이핑 시 나눔으로 | `Family-Style`로 기록, 이름→페이스 라운드트립 테스트 |
| Type3를 "임베드 완성" 경로로 보냄 | `editing permissions prohibit editable outline embedding` 실패 | 생성 Type3는 싱글 스타일 경로로 |
| 표시 레이블이 접근성 이름을 삼킴 | 업스트림/스크린리더가 `get_by_label("Times")` 실패 | 행 이름은 영문 패밀리, 노트는 표시만(`family_option`) |
| `$GITHUB_ENV`에 경로 | `Invalid format '/…/bin'` | `$GITHUB_PATH` |
| 아티팩트 glob 과매칭 | `pdfcraft-*-…tar.gz`가 cli tarball까지 잡아 단정 실패 | `pdfcraft-[0-9]*-…` |
| macOS env 쌍 | `PDFCRAFT_VERSION` 빠지면 `CFBundleVersion=0.18.0.0.18.0`·이름에 라벨 없음 | VERSION+LABEL 둘 다(Linux는 LABEL만, Windows는 VERSION에 라벨) |
| 제어 채널 단위 | 비트맵은 2×, `ui.click`/`region`은 **pt** — 창 밖 클릭은 조용히 실패 | `ui.resize`로 창 키우고 pt 좌표 사용, `ui.screenshot`은 **항상 `--out`**(없으면 base64 유출) |
| rerun의 YAML | 고쳐도 예전 동작 | 새 dispatch |
| `ditto` 병합 설치 | 옛 `.rten` 잔여물으로 strict 검증 실패 | `rm -rf` 후 복사 |
| Linux glibc 하한 | ubuntu-24.04 빌드는 22.04에서 실행 불가 | 릴리스 본문에 명시, AppImage는 헤드리스 미증명이면 생략 |
| PIL 없는 python | 픽셀 비교 실패 | `sips -s format bmp` + 표준 라이브러리로 메트릭 |
| 카탈로그 이주로 조용한 손실 | 라벨이 영어로 바뀌어도 테스트는 green | §3b, 합성 스크립트 + 명시 스킵 |
| flatpak pin 테스트 위치 | `--test it`/`--lib` 필터로 0 통과(필터가 못 만남) | `cargo test -p pdfcraft --bin pdfcraft flatpak` |
| `gh release download -p` 여러 개 | 마지막 패턴만 적용돼 일부 자산 누락 | `--pattern` 반복 + `--clobber`, 존재하면 에러 |
| `set -e` + 반한값 1 | `pkill`, 파일 존재 시 `gh download`가 스크립트를 조용히 중단 | `\|\| true` |
| 한글 IME | 스페이스 전까지 캐럿 안 밀남(보고) | 앱 아님 판정됨: `ime_text`가 런타임 증빙, 남은 건 실물 2-Set Korean 비교 |
```
