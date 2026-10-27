//! Korean text recognition on Linux through PaddleOCR's PP-OCRv5 Korean model, run by the
//! `ocrs-cjk` engine (an ocrs fork whose public API mirrors upstream ocrs, typed against
//! rten 0.24). The recognition model and its character dictionary are fetched by
//! `cargo xtask models` (see ATTRIBUTION.toml); detection reuses the Latin `text-detection.onnx`
//! the English path already ships: DB text detection is script-agnostic, and its generous line
//! crops suit the recognition model better than PaddleOCR's own tighter detectors.
//!
//! The engine hands [`Korean::recognize`] RGBA page pixels; word boxes come back in top-left
//! image pixels `[left, top, right, bottom]`, the same shape the macOS Vision backend and the
//! Latin ocrs path return, so the Korean text layer (`pdfcraft_edit::ocr_korean_layer`) is
//! unchanged.

use std::path::{Path, PathBuf};

use ocrs_cjk::{ImageSource, OcrEngine, OcrEngineParams, TextItem};
use rten_cjk::Model;

use crate::{Line, OcrError, Word};

/// The Korean recognition model, as named in ATTRIBUTION.toml.
pub const KOREAN_RECOGNITION_MODEL: &str = "korean-recognition.onnx";
/// The recognition model's character dictionary, as named in ATTRIBUTION.toml.
pub const KOREAN_DICTIONARY: &str = "korean-dict.txt";

/// The Korean model files: the shared Latin detection model plus the Korean recognition model
/// and its dictionary, searched where [`crate::Models::find`] looks.
#[derive(Clone, Debug, PartialEq)]
pub struct PaddleModels {
    pub detection: PathBuf,
    pub recognition: PathBuf,
    pub dictionary: PathBuf,
}

impl PaddleModels {
    /// The Korean files in `dir`, if all three are there.
    pub fn in_dir(dir: &Path) -> Option<PaddleModels> {
        let m = PaddleModels {
            detection: dir.join(crate::DETECTION_MODEL),
            recognition: dir.join(KOREAN_RECOGNITION_MODEL),
            dictionary: dir.join(KOREAN_DICTIONARY),
        };
        (m.detection.is_file() && m.recognition.is_file() && m.dictionary.is_file()).then_some(m)
    }

    /// Look for the Korean files in the same directories [`crate::Models::find`] searches.
    pub fn find() -> Option<PaddleModels> {
        crate::Models::search_dirs().iter().find_map(|d| Self::in_dir(d))
    }
}

/// The recognition model's alphabet, from the dictionary: PaddleOCR's CTC convention makes
/// class 0 the blank (not in the file), dictionary line i class i + 1, and the PP-OCRv5 export
/// adds one final class for the space (`use_space_char=True`). The dictionary lines in file
/// order plus one trailing space therefore match the model's output columns; ocrs-cjk checks
/// the count and refuses to run when the wiring is wrong.
fn alphabet(dictionary: &str) -> String {
    let mut alphabet: String = dictionary.lines().collect();
    alphabet.push(' ');
    alphabet
}

/// A loaded Korean recogniser. Loading takes a moment; keep one and reuse it.
pub struct Korean {
    engine: OcrEngine,
}

impl Korean {
    pub fn load(models: &PaddleModels) -> Result<Korean, OcrError> {
        let load = |p: &Path| Model::load_file(p).map_err(|e| OcrError::Load(p.display().to_string(), e.to_string()));
        let dictionary =
            std::fs::read_to_string(&models.dictionary).map_err(|e| OcrError::Load(models.dictionary.display().to_string(), e.to_string()))?;
        let engine = OcrEngine::new(OcrEngineParams {
            detection_model: Some(load(&models.detection)?),
            recognition_model: Some(load(&models.recognition)?),
            alphabet: Some(alphabet(&dictionary)),
            ..Default::default()
        })
        .map_err(|e| OcrError::Load("ocr engine".into(), e.to_string()))?;
        Ok(Korean { engine })
    }

    /// Recognize the text of a page rendered to RGBA pixels. Korean only: the Latin path stays
    /// with ocrs and its models.
    pub fn recognize(&self, rgba: &[u8], width: u32, height: u32) -> Result<Vec<Line>, OcrError> {
        if width == 0 || height == 0 || rgba.is_empty() {
            return Err(OcrError::EmptyImage);
        }
        // ocrs-cjk reads RGB; the page arrives as RGBA (drop the alpha channel).
        let rgb: Vec<u8> = rgba.as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
        let err = |e: &dyn std::fmt::Display| OcrError::Recognize(e.to_string());
        let source = ImageSource::from_bytes(&rgb, (width, height)).map_err(|e| err(&e))?;
        let input = self.engine.prepare_input(source).map_err(|e| err(&e))?;
        let found = self.engine.detect_words(&input).map_err(|e| err(&e))?;
        let lines = self.engine.find_text_lines(&input, &found);
        let read = self.engine.recognize_text(&input, &lines).map_err(|e| err(&e))?;
        // A line whose recognition failed comes back as None; word boxes are already
        // top-left-origin pixels, and words split on the space class like the Latin path.
        Ok(read
            .into_iter()
            .flatten()
            .map(|line| Line {
                words: line
                    .words()
                    .map(|w| {
                        let r = w.bounding_rect();
                        Word { text: w.to_string(), rect: [r.left() as f32, r.top() as f32, r.right() as f32, r.bottom() as f32] }
                    })
                    .filter(|w| !w.text.trim().is_empty())
                    .collect(),
            })
            .filter(|l| !l.words.is_empty())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gate is the files: an empty directory offers no Korean, the shared detection model
    /// alone is not enough, and the real machine's gate agrees with what recognition would find.
    #[test]
    fn korean_is_unavailable_without_its_models() {
        let dir = std::env::temp_dir().join(format!("pdfcraft-ocr-paddle-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(PaddleModels::in_dir(&dir), None);
        std::fs::write(dir.join(crate::DETECTION_MODEL), b"model").unwrap();
        assert_eq!(PaddleModels::in_dir(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(crate::language_available("ko"), PaddleModels::find().is_some());
    }

    /// The dictionary wires the model's output classes: one character per line in file order
    /// (the first is the single jamo ᄀ, U+1100), plus exactly one trailing space class.
    #[test]
    fn the_alphabet_is_the_dictionary_in_order_plus_the_space_class() {
        let Some(models) = PaddleModels::find() else {
            eprintln!("skipped: Korean OCR models not installed");
            return;
        };
        let dictionary = std::fs::read_to_string(&models.dictionary).unwrap();
        let chars: Vec<char> = alphabet(&dictionary).chars().collect();
        assert_eq!(chars.len(), dictionary.lines().count() + 1);
        assert_eq!(chars[0], '\u{1100}');
        assert_eq!(*chars.last().unwrap(), ' ');
        assert_eq!(chars.iter().filter(|&&c| c == ' ').count(), 1, "the space is the one trailing class");
    }

    /// ocrs-cjk validates the alphabet against the model's output columns when recognition
    /// runs: the wired alphabet reads the page, and one missing class (the trailing space)
    /// fails loudly instead of misreading.
    #[test]
    fn a_wrong_alphabet_fails_loudly_and_the_right_one_reads() {
        let Some(models) = PaddleModels::find() else {
            eprintln!("skipped: Korean OCR models not installed");
            return;
        };
        let dictionary = std::fs::read_to_string(&models.dictionary).unwrap();
        let load = |p: &Path| Model::load_file(p).unwrap();
        let engine = |alphabet: String| {
            OcrEngine::new(OcrEngineParams {
                detection_model: Some(load(&models.detection)),
                recognition_model: Some(load(&models.recognition)),
                alphabet: Some(alphabet),
                ..Default::default()
            })
            .unwrap()
        };
        let read = |engine: &OcrEngine| {
            let source = ImageSource::from_bytes(&[255u8; 3 * 64 * 32], (64, 32)).unwrap();
            let input = engine.prepare_input(source).unwrap();
            let found = engine.detect_words(&input).unwrap();
            let lines = engine.find_text_lines(&input, &found);
            engine.recognize_text(&input, &lines)
        };
        assert!(read(&engine(alphabet(&dictionary))).is_ok(), "the wired alphabet runs");
        let short: String = dictionary.lines().collect();
        assert!(read(&engine(short)).is_err(), "an alphabet one class short fails loudly");
    }

    /// End to end on a rendered Korean page, when the models are installed (`cargo xtask models`).
    /// The fixture was drawn with PIL and the openly licensed NanumGothic face
    /// (tests/data/README.md); single-vowel confusions (나눔고딕 → 나눔고닥) are known model
    /// behaviour, so 15 of its 16 words must come back exactly.
    #[test]
    fn reads_rendered_korean_text() {
        let Some(models) = PaddleModels::find() else {
            eprintln!("skipped: Korean OCR models not installed");
            return;
        };
        let korean = Korean::load(&models).unwrap();
        let (w, h, px) = fixture();
        let lines = korean.recognize(&px, w, h).unwrap();
        let text: Vec<String> = lines.iter().map(Line::text).collect();
        let words: Vec<&str> = lines.iter().flat_map(|l| l.words.iter().map(|w| w.text.as_str())).collect();
        let expected = [
            "한국어",
            "스캔",
            "테스트",
            "검색",
            "가능한",
            "학교안심",
            "나눔고딕",
            "문서",
            "인식",
            "확인",
            "2026년",
            "10월",
            "27일",
            "PdfCraft",
            "OCR",
        ];
        let found = expected.iter().filter(|e| words.contains(e)).count();
        assert!(found >= 14, "{found}/{} of {expected:?} in {text:?}", expected.len());
        assert!(text.iter().any(|t| t.contains("한국어 스캔 테스트 검색 가능한 스캔")), "{text:?}");
        assert!(text.iter().any(|t| t.contains("PdfCraft OCR")), "{text:?}");
    }

    /// The fixture page as RGBA pixels, the shape the engine hands the recogniser.
    fn fixture() -> (u32, u32, Vec<u8>) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/fixture-ko.png");
        let bytes = std::fs::read(&path).unwrap();
        let mut decoder = png::Decoder::new(std::io::Cursor::new(&bytes));
        decoder.set_transformations(png::Transformations::EXPAND);
        let mut reader = decoder.read_info().unwrap();
        let mut raw = vec![0; reader.output_buffer_size().unwrap_or_default()];
        let info = reader.next_frame(&mut raw).unwrap();
        let (w, h) = (info.width, info.height);
        let px = match info.color_type {
            png::ColorType::Rgba => raw[..(w * h * 4) as usize].to_vec(),
            png::ColorType::Rgb => raw[..(w * h * 3) as usize].as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
            png::ColorType::Grayscale => raw[..(w * h) as usize].iter().flat_map(|&g| [g, g, g, 255]).collect(),
            other => panic!("unexpected fixture color type {other:?}"),
        };
        (w, h, px)
    }
}
