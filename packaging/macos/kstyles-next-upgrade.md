# kstyles 체인 버전 업그레이드 프롬프트 v5 (그대로 붙여넣기 — 이름·번호는 에이전트가 계산)


이 문서는 `github/homeass/pdfcraft`의 비공식 한글 체인을 **지금 버전(0.19) → 다음 버전(0.20)**으로 올릴 때 쓰는 유일한 절차서다.
`kstyles14`(0.17.0)·`kstyles15`·`kstyles16`(0.18.0)·`kstyles17`(0.18.2)kstyles18`(0.19) 실제로 든 비용과 함정을 그대로 반영했다.
**아래 블록을 수정 없이 붙여넣으면 된다** — 버전·태그·브랜치·연번은 0단계에서 스스로 구한다.

```
- `github/homeass/pdfcraft`(한글 폰트·한글 OCR 체인 포크)의 한글 체인을 **현재 릴리스에서 다음 업스트림 릴리스로** 업그레이드해
 `v<다음버전>-kstyles<다음연번>`으로 배포한다. 세 플랫폼(macOS arm64 · Ubuntu x86_64 · Windows x64)은 전부 GitHub Actions
 네이티브 러너에서 빌드·검증하고 같은 태그에 부착한다(로컬 수동 산출물 금지).
- 이 Mac Studio의 `/Applications/PdfCraft.app`에 덮어쓰기 설치하고, 배포본 산출물로 실제 동작을 스모크 검증한다.
- 테스트 빌드이므로 서명·공증은 의도적 생략(ad-hoc/무서명)이다.
- 모든 게이트와 검증은 "증빙이 남는 명령"으로만 통과로 친다(로그·해시·diff·테스트 출력). 세션 md를 워크스페이스에 저장하고
 끝에 이 프롬프트를 최신 상태로 갱신해 남긴다.

## 0) 변수 자동 계산 (사람이 고치는 항목 없음)
REPO=/Volumes/ExternalSSD/Code/work/pdfcraft (재클론·새 워크트리 금지 — target/ 캐시 유지. 워크트리 경로는 반드시 /tmp 절대경로,
 리비트는 refs/remotes/<remote>/<branch> 한정 표기 — 저장소 안에 `origin/main` 같은 경로가 생기면 push가 엉뚱하게 실패한다)
`git fetch --all --tags --prune` 후 아래를 계산해서 보고에 적는다:
- CUR_TAG  : 가장 최근 `v*-kstyles*` 태그가 가리키는 업스트림 베이스 (`git rev-parse <tag>^{}` 의 부모 사슬에서 version 확인)
- NEXT_TAG : `git tag | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1` 이 CUR보다 새로우면 그것. 없으면 업그레이드 불요 — 중단하고 보고
- PREV_KSTYLES / NEXT_KSTYLES: `git tag | grep kstyles | sort -V | tail -1` 의 번호 + 1 (예: kstyles17 → kstyles18)
- PREV_BRANCH: 그 태그가 물려 있는 브랜치 (`git branch -r --contains <tag>` 로 찾되, `feat/*-kstyles*` 중 최신 것)
- BRANCH  : `feat/<NEXT_TAG의 major.minor>-kstyles<NEXT_KSTYLES>` (예: feat/18.4-kstyles18)
- BASE   : **베이스 후보를 검증**한다 — `git merge-base --is-ancestor $NEXT_TAG refs/remotes/upstream/main`
  - YES → BASE=refs/remotes/origin/main (포크 main을 upstream/main과 동기화하고 그 위에서 진행)
  - NO → 릴리스 태그가 trunk에 없는 경우(0.18.2가 그랬다). **오너에게 "릴리스 태그 vs trunk"를 물어 확정**하고,
      trunk 선택이 아니면 포크 main은 건드리지 않는다(워크플로가 이미 main에 등록돼 있어 dispatch에는 지장 없음).
- MB    : `git merge-base refs/remotes/origin/$PREV_BRANCH $BASE` (재이식 지점. 이 값과 커밋 수를 반드시 보고에 적는다)
- 콘텐츠 규모는 커밋 메시지로 추론하지 말 것: `git show --stat $NEXT_TAG` — "Release 0.18.2" 한 커밋이 186파일 +22,684/−1,874였다.

## 1) 겹침 사전 계산과 레인 배치
- 체인이 건드린 파일 ∩ 업스트림이 건드린 파일:
 `comm -12 <(git diff --name-only $MB..refs/remotes/origin/$PREV_BRANCH|sort) <(git diff --name-only <직전베이스> $NEXT_TAG|sort)`
- 레인: ①구현(재이식·충돌·게이트) ②리서치(겹침 파일별 "같은 함수/헝" 분류, 새 게이트, 이사·삭제 감지) ③오케스트레이터(저는 제 손으로
 게이트·산출물·라이브 확인을 재실행) 를 **병렬**로 돌린다. 리서치 결과가 구현보다 먼저 함정을 지적한 적이 많다.

## 2) 재이식
`git checkout -b $BRANCH refs/remotes/origin/$PREV_BRANCH` → `git rebase --onto $BASE $MB`
- 스쿼시 금지, 업스트림 커밋 재이식 금지(`--no-merges` 포트 커밋 수와 대조해 맞는지 확인).
- 같은 파일이 2연속 의미 충돌하면 스쿼시 3-way apply(`git diff $MB..origin/$PREV_BRANCH`를 $BASE에)로 전환하고 그 사실을 보고한다.
- 업스트림이 우리 기능을 흡수한 커밋이 있으면 그 커밋만 버린다(`git log <직전베이스>..$NEXT_TAG --grep`으로 확인).

## 3) 조용한 회귀(silent loss) — 반드시 개별 증명
자동 병합돼도 기능이 죽는다. 아래는 실제로 발생했던 항목들이라 **매 사이클마다** 증명한다:
- 각 실충돌 파일: `git diff $NEXT_TAG -- <파일>`이 **포트 추가분만** 보여주는지(업스트림 리팩터가 포트 코드를 삼킨 사례: kstyles15 `format_text_ui.rs`의 Tinos 프리뷰 — 테스트는 green이었음)
- 에디터 프리뷰 = 저장 서체: `the_editor_preview_draws_hangul_with_the_written_korean_face`·`…follows_family_weight_and_slant`
- 재타이핑 서체 보존: `korean_face_survives_a_retype_without_a_font_request`, `retyping_added_korean_text_keeps_its_face_through_tools`,
 `retyping_an_added_korean_paragraph_in_the_editor_keeps_its_face` (Type3 `/FontName`은 `Family-Style` 형식 유지 — 스타일이 빠지면
 이름→페이스 역조회가 조용히 None이 된다), 생성 Type3은 "임베드 프로그램 완성" 경로 금지(fsType 거부)
- 선택기 접근성 이름은 영문 패밀리(`family_option`), 한글 안내는 표시만 — 이걸 잃으면 업스트림 테스트가 깨진다
- **카탈로그/자산의 "이사"를 매번 의심**: 체인이 편집하던 파일이 새 베이스에 없으면(`git log --diff-filter=D -- <path>`) 데이터가 나간 것.
 0.18.2는 i18n 카탈로그를 `craft-i18n`(flatpak이 커밋을 핀)으로 옮겨 우리 한글 라벨 4행이 죽은 데이터가 되었고 `tr()` 영어 폴백으로
 테스트는 green이었다. 대응 패턴: 업스트림 핀·flatpak 핀을 **만지지 않고** 로컬 합성
 (`packaging/craft-i18n-korean-merge.sh` → `CRAFT_I18N_DIR`), 없으면 스킵 사유를 출력하도록 테스트를 고친다.
 새 합성 스크립트를 만들 땐 반드시 **빌드 산출물에 문자열이 들어가는지**로 끝까지 증명한다(단위 테스트 통과로는 부족).
- 자동 병합된 파일은 심볼 공존 검사(예: `korean_fallback`+`jbig2_work`, `is_korean`+`dict_in_place`, `"ko"`+`inherit`+`images_refused`,
 한글 렌더 테스트+신규 렌더 테스트, `mark_transactions`+한글 편집 테스트, parity 한글 행)

## 4) 생성파일 규칙
`ATTRIBUTION.md`는 `cargo xtask assets --write`로만(이름상 `ci`에 없지만 `license-check`가 전체 에셋 검증을 돌려 실효 게이트다;
0.18.2에서 번역 카탈로그의 sha 면제가 폐지되어 카탈로그 항목이 남으면 실패). `NOTICE`와 `LICENSES.toml`의 NOTICE 해시는 한 세트로
이동(`--write` 없음 — 업스트림이 해시 대상 문서를 바꿨을 때만 1줄 재감사, 이전 해시는 주석 보존). `Cargo.lock` 손편집 금지.

## 5) 게이트 (매 명령 `export PATH=$HOME/.cargo/bin:$PATH` — Homebrew rustc는 rust-version 미달.
##   `CRAFT_FONTS_DIR`+`CRAFT_FONTS_REQUIRED=1`, 카탈로그 합성이 있으면 `CRAFT_I18N_DIR`도. 양쪽 on/off 모두로 도는 것 포함)
`cargo check --workspace --all-targets` ·
`cargo check -p pdfcraft-ocr --target aarch64-apple-darwin --locked`(세 패키징 스크립트 모두 `--locked`) ·
`cargo test --release -p pdfcraft-render -p pdfcraft-edit --lib -- korean` · `cargo test --release -p pdfcraft-ocr --lib` ·
`cargo test -p pdfcraft-ui-egui --test it editing` · `--test it korean` · `--test it fonts` · `--test it ime_text` · `--lib i18n` ·
엔진 e2e `ocr_korean`(Vision 실호출, skip 아니어야 함) · 자동화 `retyping*` ·
`cargo test -p pdfcraft --bin pdfcraft flatpak`(핀 짝 테스트 — 하비가 `--bin`이라 필터로 못 찾기 일쑤) ·
`cargo fmt --all -- --check` · `cargo clippy -p pdfcraft-ui-egui -p pdfcraft-edit -p pdfcraft-render -p pdfcraft-automation --all-targets -- -D warnings` ·
`cargo xtask assets`+`license-check`+`parity` · `actionlint`+`bash -n`(+ 워크플로 수정 시)

## 6) 표시/저장 이중 실측 (제가 직접, 레인 보고에 의존하지 않음)
- 저장: `page_add_text font=helvetica|times|courier` → `text_lines`가 `NanumGothic-Regular`/`HakgyoansimSangjangR-Regular`/`HakgyoansimChulseokbuTTF-Light`
 → `pdfcraft-cli render` PNG 3종 해시가 서로 다름
- 재타이핑 보존: `page_add_text font=times` → `text_edit`(font 인자 **없이**) → `text_lines`가 원본 페이스 유지
- 표시: 앱을 `--control`로 띄워 `ui.resize` → `edit.text` → 캔버스 클릭(pt 단위! 비트맵은 2×) → `ui.click label="Font"` →
 `ui.click label=Times|Courier|Helvetica`(행 클릭 성공을 위젯 id·콤보 값으로 확인) → `ui.type` → `ui.screenshot --out`** 필수**
 → 3종 해시 상이 + 시각 판정(눈으로 명조/고딕/타이피라이터 구별)
- 카탈로그 라벨: 빌드된 실행 파일에서 `나눔고딕`·`학교안심 상장`… 문자열 검출(임베드 증명) + `tr()` 해석 테스트

## 7) 배포는 편집이 아니라 dispatch
워크플로는 `tag`/`ref`/`version`/`label` **입력**을 받는다 → 새 연번은 코드 편집 없이 dispatch로 만든다:
`gh workflow run kstyles-macos.yml|kstyles-linux-deb.yml|kstyles-windows-zip.yml --ref main -f tag=v<다음>-kstyles<다음연번> -f ref=$BRANCH [-f version=… -f label=…]`
- 단 dispatch는 **기본 브랜치(main)의 YAML**을 읽는다. 워크플로를 고쳤으면 반드시 main에도 반영하고,
 **`gh run rerun`은 원본 커밋의 YAML을 쓰므로** 수정 후엔 새 dispatch를 쓴다.
- `gh release download`는 `-p`를 여러 번 주면 마지막만 적용된다 → `--pattern` 반복 + 필요시 `--clobber`.
 `set -e` 아래 `pkill`·파일존재 시 `gh`의 반환값 1이 스크립트를 조용히 중단한다 → `|| true`.
- 커밋 이메일은 author/committer 모두 `biometal.dr@gmail.com`(GH007). 리베이스 후 `git log --format='%ce' $BASE..HEAD | sort -u`로 확인,
 어긋나면 `filter-branch --env-filter`로 정규화(트리는 0줄 차이여야 함) 후 재조립.
- 태그는 **빌드될 브랜치 팁**에 annotated로 만들고 push, 그 뒤에 릴리스 생성(자산 없이) → dispatch → 자산 부착.

## 8) 설치·확인 (Mac Studio)
- 새 백업: 직전 판을 `pdfcraft-kstyles<직전연번>-backup/`로 보존(영어 정식판·kmacos2 백업과 함께 유지).
  백업 직후 `xattr -dr com.apple.quarantine <백업>` + 버전 확인까지 해야 완료 —
  `ditto`가 quarantine을 그대로 복사해, 안 지우면 백업 앱 실행 시 Gatekeeper가 SIGKILL(137)한다
  (`codesign --verify --strict`는 통과하므로 속기 쉽다).
- 덮어쓰기: `rm -rf /Applications/PdfCraft.app && ditto <mounted>/PdfCraft.app /Applications/` (plain `ditto`는 병합 → 옛
 `Contents/Resources/models/*.rten` 잔여물이 `codesign --verify --strict`를 깬다)
- 확인: `--version` == `pdfcraft <버전> (kstylesNN)`, `CFBundleVersion` == `<버전>.kstylesNN`, strict 검증, `Resources/models` 4종+라이선스,
 실행(PID) 확인, 배포본 CLI(`PDFCRAFT_MODELS=/Applications/PdfCraft.app/Contents/Resources/models`)로 Vision 한글 스모크
 (`/private/tmp`등에 픽스처 재생성 — /tmp는 며칠 만에 정리된다), 저장 파일에서 한글 재추출
- 산출물 무결성: 릴리스에서 받아와 `sha256`을 저장소측 `digest`와 대조, `SHA256SUMS.txt`로 `-c`
- 잔여물 검사: `find /Applications/PdfCraft.app -name '*.rten*'`가 0개, 모델 4종+라이선스 존재, `codesign --verify --strict --deep`

## 9) 문서·마무리
- 각 자산 sha256을 `gh release view --json assets`의 `digest`로 교차 확인해 본문·README·`SHA256SUMS.txt`(자기 자신 제외)에 반영
- 세션 md: `PdfCraft-kstyles<NN>-<버전>-재이식-세션기록.md` + 디버그 리포트(검증 실패·방법 실수 포함)를 워크스페이스와 `Code/results/`에 저장,
 **끝에 이 프롬프트를 최신 버전으로 갱신**해 넣는다. 저장소 플레이북(이 파일)도 같은 커밋 범위에서 갱신.

## 알려진 함정 (kstyles14·15·16·17 실측)
| 함정 | 증상 | 대비 |
|---|---|---|
| 프리뷰가 저장 서체를 안 따라감 | 한글이 항상 나눔고딕으로 보임(저장은 정상) | 에디터 폰트 스택에 스타일 헤드 한글(`korean_writer_face`/`korean_preview_family`) |
| Type3 `/FontName`에 스타일 없음 | 이름→페이스 역조회가 조용히 None → 재타이핑 시 나눔 | `Family-Style`로 기록 + 라운드트립 테스트 |
| Type3를 "임베드 완성" 경로로 보냄 | `editing permissions prohibit editable outline embedding` | 생성 Type3는 싱글 스타일 경로 |
| 카탈로그 이주 | 라벨이 영어로 바뀌어도 테스트는 green | §3 이사 감지 + 로컬 합성(`CRAFT_I18N_DIR`) + 산출물 문자열 증명 |
| 표시 레이블이 접근성 이름을 삼킴 | `get_by_label("Times")` 실패(업스트림 테스트 파손) | 행 이름은 영문 패밀리, 안내는 표시만(`family_option`) |
| 베이스가 main에 없음 | "업스트림 머지"로 다른 내용 위로 재이식 | §0 `--is-ancestor` 검사 후 오너 결정 |
| "Release x.y.z"가 콘텐츠 커밋 | 드리프트 규모 오판 | `git show --stat <태그>` |
| 생성파일까지 업스트림이 변경 | `assets`/`license-check` 실패, sha 면제 폐지 | §4 규칙, 재생성만 허용 |
| `$GITHUB_ENV`에 경로 | `Invalid format '/…/bin'` | `$GITHUB_PATH` |
| 아티팩트 glob 과매칭 | `pdfcraft-*-…tar.gz`가 cli tarball까지 단정 실패 | `pdfcraft-[0-9]*-…` |
| macOS env 쌍 | `PDFCRAFT_VERSION` 빠지면 `CFBundleVersion=0.18.0.0.18.0`·이름에 라벨 없음 | VERSION+LABEL 둘 다(Linux는 LABEL만, Windows는 VERSION에 라벨) |
| dispatch는 main YAML | 브랜치에서 고쳐도 CI는 옛 단계로 빌드(라벨 합성이 CI에서 빠지는 등) | 워크플로 변경 시 main 반영 + 새 dispatch |
| 제어 채널 단위·인자 | 창 밖 클릭은 조용히 실패, `--out` 없으면 base64 유출 | pt 좌표·`ui.resize`, `screenshot --out` 필수 |
| `gh`/`pkill` 반환값 | `set -e` 아래 단계가 조용히 중단 | `--clobber`/`|| true` |
| `/tmp` 정리 | 며칠 뒤 픽스처·검증 산출물 소멸 | 매 사이클 재생성 스크립트 보유 |
| `ditto` 병합 설치 | 옛 `.rten` 잔여물로 strict 검증 실패 | `rm -rf` 후 복사 |
| Linux glibc 하한 | ubuntu-24.04 빌드는 22.04에서 실행 불가 | 본문에 명시, AppImage는 미증명이면 생략 |
| PIL 없는 python | 픽셀 비교 실패 | `sips -s format bmp` + 표준 라이브러리 |
| `gh run watch` 조기 종료 | 잡이 `in_progress`인데 exit 0 + `conclusion: ""` → 자산 없는 걸 성공으로 오인 | 상태 폴링(`--json status`가 `completed`일 때까지) + `gh release view --json assets`로 실제 부착 확인 |
| 카탈로그 임베드 위치 | CLI zip에서 라벨 0개 → 합성 누락으로 오판 | GUI 앱 바이너리(`Contents/MacOS/PdfCraft`)에서 검사 |
| `/tmp` 정리 + PIL 없는 python | 며칠 뒤 검증 픽스처·Pillow가 없어 스모크 실패 | 검증에는 저장소 픽스처 `crates/ocr/tests/data/fixture-ko.png`를 쓰고, 재생성이 필요하면 고정 스크립트를 워크스페이스에 보관 |
| 한글 IME | 스페이스 전까지 캐럿이 안 밀린다는 보고 | `ime_text`가 앱 경로 정상 증명 → 플랫폼 측, 실물 키보드 절차로 승부 |
```

## 상태 스냅샷 (2026-11-10)
`v0.19.0-kstyles18`(브랜치 `feat/19.0-kstyles18`, 베이스 태그 `v0.19.0`) 배포 완료 — 3종 네이티브 CI success, 릴리스 자산 10개 + README·SHA 완비.
다음 사이클은 `kstyles19`(업스트림 `v0.20.0` — 이미 릴리스됨)이다.
