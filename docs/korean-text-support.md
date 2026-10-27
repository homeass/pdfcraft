# Korean text support

> **Last reviewed:** 2026-10-27 · **Last updated:** 2026-10-27 · **Change:** minor (Scan & OCR reads Korean scans on macOS 13+ via the system Vision framework) · **Scope:** rendering, interface fallback, editing and OCR of Korean (Hangul) text

What PdfCraft can and cannot do with Korean text today, which font carries it, and where each
behaviour is pinned by a test. Korean interface *translation* (a `ko` catalog) is separate work:
see [localization-parity.md](localization-parity.md) and gap #14 in [gaps.md](gaps.md).

## The font

**나눔고딕 / NanumGothic** (Naver, OFL-1.1) is the **default Korean face** — the one plain
Helvetica text, the Korean-font error message and the web build name. It lives in
[storytold/craft-fonts](https://github.com/storytold/craft-fonts) like every shared font
(`AGENTS.md` §1.4) as a Regular TrueType covering the full U+AC00–U+D7A3 syllable block plus
modern and compatibility jamo. The default is picked by name (`korean_default_face`: NanumGothic
Regular, then any NanumGothic, then Hakgyoansim Namu Regular, then any Namu, then the first Kore
Regular), never by manifest order.

**학교안심 나무 / Hakgyoansim Namu** (KERIS, designed by Design210, OFL-1.1) keeps the fallback
role: still selectable in the pickers, still in the Kore fallback chain behind the default, and
the default again whenever a checkout has no NanumGothic. Regular and Light TrueType, the full
11 172-glyph syllable block plus jamo. Its licence file travels with the package and with
`CRAFT_FONTS_DIR` releases (`packaging/env.sh`, `copy_font_licences`).

**Provenance (Namu).** The committed bytes (SHA-256 in `fonts/manifest.txt`) were downloaded on
2026-10-22 from KERIS's own official release flow — `copyright.keris.or.kr/wft/fntDwnld`
(font id `GFT202312110000000000013`, government CDN `font-real.cdn.gov-ntruss.com`, folders
`2023-12-11/1049` and `1054`) — and compared byte-for-byte: identical. The craft-fonts
ATTRIBUTION row records this; re-verify against a fresh official download if the file is ever
replaced.

The script tag is `Kore` (ISO 15924). `crates/fonts/build.rs` embeds the faces with every other
manifest face under `CRAFT_FONTS_DIR` (on the web: NanumGothic and Chulseokbu Light only);
builds without the checkout keep working and every consumer below
degrades to a clear message, never to wrong glyphs.

## What works (each line cites its test)

| Path | Behaviour | Evidence |
|---|---|---|
| Interface fallback | `ui_korean_fonts()` joins the egui family chain after CJK/Arabic/Telugu/Hebrew/Thai, so file names and titles in Hangul draw real glyphs | `crates/ui-egui/tests/fonts.rs::korean_ui_text_uses_craft_fonts` |
| Viewing a PDF | a non-embedded CID font whose `/CIDSystemInfo /Ordering` is `Korea1`/`KR` substitutes the Korean face and says so (`Why::ScriptFace`); rasterization of such pages draws with it | `crates/fonts/src/fallback.rs::korean_depends_on_craft_fonts_and_says_which_it_got`, `crates/render/src/raster.rs::non_embedded_korean_cid_fonts_draw_with_a_craft_fonts_face` |
| Editing existing text | Hangul replacement text keeps its own face on the line (Latin and digits too, one look per line), generated as one-glyph-per-character Type 3 fonts named `PCKo…` / `/Name PCKorean`; characters the face lacks are taken from the Japanese faces; longer runs split across fonts and still read as one line; the source font is reused whole when it already covers every character | `crates/edit/src/tests.rs::korean_replacement_text_uses_the_korean_face`, `crates/edit/src/tests.rs::korean_replacement_without_a_korean_face_says_so`, `crates/automation/tests/automation.rs::editing_existing_korean_text_through_tools` |
| Add content ▸ text | a new Added-text item draws Hangul with the same Kore faces and one-glyph-per-character Type 3 mechanism, in the style's own face (plain Helvetica → NanumGothic, Times → Sangjang R, Courier → Chulseokbu Light — no Korean mono face exists, bold → Chulseokbu Bold), while Latin and digits in the same box keep the item's standard font; widths come from the drawn glyphs, so wrap/align/justify work, and a build without the faces refuses with the Korean-font message | `crates/edit/src/tests.rs::added_korean_text_is_written_and_read_back`, `crates/edit/src/tests.rs::added_korean_text_draws_the_style_the_request_asks_for`, `crates/edit/src/tests.rs::added_korean_text_without_a_korean_face_says_so`, `crates/automation/tests/automation.rs::adding_korean_text_through_tools` |
| Embedding | any full Unicode face including Hangul round-trips through `pdfcraft_fonts::embedding::embed` (identity `PCFullFont` program, UCS-2 CIDs, ToUnicode) | `crates/fonts/src/embedding.rs::korean_text_round_trips_through_the_korean_craft_face` |
| CLI demo | `cargo run -p pdfcraft-fonts --example korean -- out.pdf` writes a Hangul page with the embedded face | manual; covered programmatically by the embedding test above |
| Scan & OCR ▸ Recognize text | on **macOS 13+** the Document-language dropdown offers Korean; each page is read by the system Vision framework (on-device, no models, no network) and the Hangul words are laid back over the picture as invisible `PCKo…` Type 3 with the face's ToUnicode, so a Korean scan becomes searchable and selectable. Vision returns one line per text block (no per-word boxes), so each word's box splits the line in proportion to its characters | `crates/ocr/src/vision.rs` (unit: box mapping), `crates/engine/tests/it/ocr_korean.rs::korean_scan_becomes_searchable_through_vision`, `crates/edit/src/tests.rs::ocr_korean_layer_is_invisible_searchable_and_skips_undrawable_words` |

Parity entries updated: `view.render-cid-fonts` and `edit.text-reflow` in
`parity/acrobat-features.toml`.

## Limits and open ends

- **Decomposed jamo**: replacement and added text draw one Type 3 glyph per character, so a
  decomposed jamo sequence (e.g. U+110B U+110B) draws as individual jamo instead of composing;
  precomposed syllables — every normal Korean input method's output — are exact.
- **Japanese Add Text** is still refused (Japanese has no shaped writer in the Added-text path
  yet); Korean's wiring there does not cover it.
- **One design**: Namu is a display-weight text face with only Regular and Light; NanumGothic
  ships Regular only. Missing-bold requests resolve by NFSS to those faces; no synthetic bold is
  drawn. The serif role is Sangjang R (Regular only); Chulseokbu carries Light and Bold.
- **GB1 / CNS1** (Chinese orderings) still substitute nothing: the only openly licensed faces
  that would cover them are Source Han / Noto CJK, excluded by `AGENTS.md` §1.1.
- **A Korean UI catalog** (`ko`) exists; the face-name rows (`나눔고딕`, `학교안심 나무`, …) are
  proper-noun lines, not translations.
- **OCR is macOS-only and line-granular**: the ocrs models read the Latin alphabet, so Korean
  recognition runs only on macOS 13+ through the system Vision framework. It gives no per-word
  boxes (each recognized word's box splits its line by character count), and a word with a
  character no Korean face holds is left out of the invisible layer rather than written as `?`.
  A build without the Kore faces, or before macOS 13, offers English only.
- NAMU's name table leaves the licence field empty (KERIS ships it that way), which is fine for
  craft-fonts embedding and `validate()`; the user-picks-a-font document-input validator
  (`validate_document_input`) requires the name table to declare its licence, so Namu chosen
  *there* would be refused — an upstream reporting item, not a blocker for the paths above.

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-26 | minor | The default Korean face moved to NanumGothic (picked by name, never manifest order); Namu keeps the fallback role; the web build embeds NanumGothic + Chulseokbu Light |
| 2026-10-22 | major | Created. Korean wired into the interface chain, CID substitution, renderer, editor Type 3 replacement and embedding; KERIS byte-match recorded |
| 2026-10-22 | minor | Korean wired into Add content ▸ text (the Added-text path): the style's own Kore faces, one glyph per character, standard-font Latin in the same box |
