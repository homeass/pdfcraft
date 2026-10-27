//! Scan & OCR of Korean on macOS: a page of rendered Hangul is fed to the recognizer as a
//! "scan" (an image-only PDF), recognized through the Vision framework, and the invisible
//! Type 3 layer it writes comes back through text extraction — the page is searchable.
//! Skipped everywhere the recognizer is not: non-macOS builds, macOS before 13 (Vision's
//! Korean revision), a build without the craft-fonts Korean faces, and a machine without the
//! OCR models the engine loads first (`cargo xtask models`).

use pdfcraft_engine::ocr::{OcrSettings, PlacedWord};
use pdfcraft_engine::{AddedText, Edit, Session, export};

/// The words the synthetic scan shows.
const KOREAN: &str = "한국어 스캔";

#[test]
fn korean_scan_becomes_searchable_through_vision() {
    if !(pdfcraft_engine::ocr::language_available("ko") && pdfcraft_engine::ocr::available() && pdfcraft_fonts::document_korean_font().is_some()) {
        eprintln!("skipped: Korean recognition needs the macOS build, the OCR models and the craft-fonts Korean faces");
        return;
    }
    // 1. A page with real Korean glyphs, added at a big size (Vision reads it best above 20 px).
    let mut s = Session::new();
    let bytes = s.create_from_text("k", "sample").unwrap();
    let id = s.open("k.pdf", None, bytes, None).unwrap();
    s.apply(
        id,
        Edit::AddText { page: 0, text: AddedText { rect: [54.0, 540.0, 500.0, 620.0], text: KOREAN.into(), size: 40.0, ..AddedText::default() } },
    )
    .unwrap();

    // 2. Rendered at 150 dpi, the page becomes a picture-only PDF: the "scan".
    let png = {
        let mut ex = export::Exporter::new(s.get(id).unwrap());
        ex.png(0, 150.0).unwrap()
    };
    let scan = s.create_from_images(&[("scan.png".into(), png)]).unwrap();
    let mut r = Session::new();
    let scan_id = r.open("scan.pdf", None, scan, None).unwrap();
    assert!(r.get(scan_id).unwrap().text_lines(0).is_empty(), "the scan is a picture only");

    // 3. Recognize it as Korean; the run reads the Hangul and applies the invisible layer.
    let found = r.recognize_text(scan_id, &[], OcrSettings { language: "ko".into(), ..Default::default() }).unwrap();
    let text: String = found.iter().map(|p| p.text()).collect::<Vec<_>>().join(" ");
    assert!(text.contains("한국어") && text.contains("스캔"), "vision read: {text:?}");

    // 4. The applied layer is searchable: extraction finds the Hangul on the picture page.
    let drawn: Vec<&PlacedWord> = found.iter().flat_map(|p| &p.words).collect();
    assert!(!drawn.is_empty(), "words were placed");
    let joined: String = r.get(scan_id).unwrap().text_lines(0).iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join(" ");
    assert!(joined.contains("한국어"), "the invisible layer extracts back: {joined:?}");
}
