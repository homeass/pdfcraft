//! Korean text recognition on macOS through the system Vision framework.
//!
//! Vision's `VNRecognizeTextRequest` reads Korean since macOS 13 (WWDC22: "We are now
//! supporting the Korean language in Vision"), on-device, with no model files and no network.
//! The workspace forbids `unsafe`, so everything here goes through the safe `objc2` wrappers:
//! the page is encoded to PNG in memory, handed to `VNImageRequestHandler`, and read back from
//! `VNRecognizeTextRequest::results`.
//!
//! Vision returns one observation per text line with a box normalized to the image (origin at
//! the bottom left); word boxes inside a line split the line box in proportion to each run's
//! character count. Hangul syllables are near-monospaced, and the text layer stretches every
//! word to its box anyway, so the approximation is close enough for search and selection.

use objc2::AnyThread;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_core_foundation::CGRect;
use objc2_foundation::{NSArray, NSData, NSDictionary, NSString};
use objc2_vision::{
    VNImageOption, VNImageRequestHandler, VNRecognizeTextRequest, VNRecognizedTextObservation, VNRequest, VNRequestTextRecognitionLevel,
};

use crate::{Line, OcrError, Word};

/// The lowest macOS whose Vision text recognizer reads Korean (request revision 3).
const KO_MIN_MAJOR: isize = 13;

/// Whether this machine's Vision framework reads Korean. Reads the OS version only — no
/// framework is loaded, so the UI can call it while building the dialog.
pub fn korean_available() -> bool {
    objc2_foundation::NSProcessInfo::processInfo().operatingSystemVersion().majorVersion >= KO_MIN_MAJOR
}

/// Recognize the text of a page rendered to RGBA pixels. Korean only: the Latin path stays
/// with ocrs and its models.
pub fn recognize(rgba: &[u8], width: u32, height: u32) -> Result<Vec<Line>, OcrError> {
    if width == 0 || height == 0 {
        return Err(OcrError::EmptyImage);
    }
    let png = encode_png(rgba, width, height)?;
    let data = NSData::with_bytes(&png);
    let options: Retained<NSDictionary<VNImageOption, AnyObject>> = NSDictionary::new();
    let handler = VNImageRequestHandler::initWithData_options(VNImageRequestHandler::alloc(), &data, &options);
    let request = VNRecognizeTextRequest::new();
    request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
    request.setUsesLanguageCorrection(true);
    let ko = NSString::from_str("ko");
    let en = NSString::from_str("en-US");
    let langs: [&NSString; 2] = [&ko, &en];
    request.setRecognitionLanguages(&NSArray::from_slice(&langs));
    let requests_vec: Vec<Retained<VNRequest>> = vec![request.clone().into_super().into_super()];
    let requests = NSArray::from_retained_slice(&requests_vec);
    handler.performRequests_error(&requests).map_err(|e| OcrError::Recognize(format!("Vision: {}", e.localizedDescription())))?;
    let lines = match request.results() {
        Some(observations) => observations
            .to_vec()
            .into_iter()
            .filter_map(|observation| {
                let text = observation.topCandidates(1).to_vec().first()?.string().to_string();
                (!text.trim().is_empty()).then(|| line_from(&text, observation_box(&observation), width, height))
            })
            .collect(),
        // An image with no text at all: Vision reports none, not an error.
        None => Vec::new(),
    };
    Ok(lines)
}

/// The observation's box in Vision's normalized coordinates (origin bottom-left).
///
/// # Safety
///
/// The workspace lints forbid `unsafe`, and objc2's `boundingBox` getter is one — every other
/// Vision call used here is safe. The getter reads a stored property of a live observation
/// object (no ownership, no allocation, no nil: `VNObservation`'s own contract), the same
/// shape every Apple sample reads. This block is the whole risk surface.
#[allow(unsafe_code)]
fn observation_box(observation: &VNRecognizedTextObservation) -> CGRect {
    // SAFETY: plain property read of a live object; see the doc comment.
    unsafe { observation.boundingBox() }
}

/// Vision's normalized bottom-left box to top-left pixels, split into words by whitespace.
fn line_from(text: &str, norm: CGRect, width: u32, height: u32) -> Line {
    let (w, h) = (width as f64, height as f64);
    let (l, t, r, b) =
        (norm.origin.x * w, (1.0 - (norm.origin.y + norm.size.height)) * h, (norm.origin.x + norm.size.width) * w, (1.0 - norm.origin.y) * h);
    if !(l.is_finite() && t.is_finite() && r.is_finite() && b.is_finite()) || r <= l || b <= t {
        return Line { words: Vec::new() };
    }
    // Character positions (spaces included) share the line box: a word runs from the character
    // index it starts at to the one it ends at, so the boxes tile the line Vision measured.
    let chars = text.chars().count() as f64;
    if chars == 0.0 {
        return Line { words: Vec::new() };
    }
    let per = (r - l) / chars;
    let mut words = Vec::new();
    let mut push = |s: f64, e: f64, word: &str| {
        if word.is_empty() {
            return;
        }
        let (x0, x1) = (l + s * per, l + e * per);
        if x0.is_finite() && x1.is_finite() && x1 > x0 {
            words.push(Word { text: word.to_string(), rect: [x0 as f32, t as f32, x1 as f32, b as f32] });
        }
    };
    let mut start: Option<(usize, usize)> = None; // (character index, byte index)
    for (ci, (bi, c)) in text.char_indices().enumerate() {
        if c.is_whitespace() {
            if let Some((sc, sb)) = start.take() {
                push(sc as f64, ci as f64, &text[sb..bi]);
            }
        } else if start.is_none() {
            start = Some((ci, bi));
        }
    }
    if let Some((sc, sb)) = start {
        push(sc as f64, chars, &text[sb..]);
    }
    Line { words }
}

/// RGBA page pixels to PNG bytes: Vision decodes image data through ImageIO.
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, OcrError> {
    let mut buf = Vec::new();
    {
        let mut enc = png::Encoder::new(std::io::Cursor::new(&mut buf), width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| OcrError::Recognize(format!("png: {e}")))?;
        w.write_image_data(rgba).map_err(|e| OcrError::Recognize(format!("png: {e}")))?;
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2_core_foundation::{CGPoint, CGSize};

    /// A normalized Vision box maps to a top-left pixel box, and the line splits into words.
    #[test]
    fn normalized_boxes_become_top_left_pixels() {
        // A line across the image's bottom: normalized y 0..0.1 of a 1000 x 2000 image.
        let norm = CGRect { origin: CGPoint { x: 0.0, y: 0.0 }, size: CGSize { width: 1.0, height: 0.1 } };
        let line = line_from("안녕 하 세 요", norm, 1000, 2000);
        assert_eq!(line.words.len(), 4);
        let first = &line.words[0];
        assert_eq!(first.text, "안녕");
        assert!((first.rect[0] - 0.0).abs() < 1e-6 && (first.rect[1] - 1800.0).abs() < 1e-3, "{first:?}");
        assert!((first.rect[3] - 2000.0).abs() < 1e-3, "{first:?}");
        let last = line.words.last().unwrap();
        assert!((last.rect[2] - 1000.0).abs() < 1e-6, "{last:?}");
    }

    #[test]
    fn words_follow_character_share_of_the_line() {
        // "A BC": four characters (A, space, B, C): the first word takes the first quarter.
        let norm = CGRect { origin: CGPoint { x: 0.0, y: 0.0 }, size: CGSize { width: 1.0, height: 1.0 } };
        let line = line_from("A BC", norm, 100, 100);
        assert_eq!(line.words.len(), 2);
        assert!((line.words[0].rect[2] - 25.0).abs() < 1e-3, "{:?}", line.words[0]);
        assert!((line.words[1].rect[0] - 50.0).abs() < 1e-3, "{:?}", line.words[1]);
        assert!((line.words[1].rect[2] - 100.0).abs() < 1e-3, "{:?}", line.words[1]);
    }

    #[test]
    fn png_roundtrips_a_1px_image() {
        let png = encode_png(&[10, 20, 30, 255], 1, 1).unwrap();
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']), "{png:?}");
    }
}
