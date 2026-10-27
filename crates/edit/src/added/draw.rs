use super::layout::{
    Piece, arabic_error, hebrew_error, is_arabic, is_hebrew, is_thai, korean_error, korean_faces, latin_width, norm, shaped_layout, text_rect,
    thai_error, wrapped,
};
use super::types::{AddedText, Align, Family, Ink};
use super::{
    Content, CraftFont, Dict, Document, EditError, GlyphError, GlyphOutline, Object, PdfString, ShapedCluster, ShapedScript, align_code, check,
    check_on_top, craft_advance, craft_has, japanese_glyph_from, literal, matrix_number, n, page_list, place_tagged, shaped_glyph, win_ansi,
};
use pdfcraft_fonts::shaped_text::{self, ShapedFonts};
use crate::text::is_korean;
use std::fmt::Write as _;

fn font_name(base: &str) -> String {
    format!("PCF{}", base.replace('-', ""))
}

#[cfg(test)]
pub(super) use pdfcraft_fonts::shaped_text::GLYPHS_PER_FONT;

/// A line of a shaped-text item after layout: its text, whether its paragraph runs right to left,
/// its paragraph, its pieces in drawing order and its width at the item's size.
pub(super) type LaidLine = (String, bool, usize, Vec<Piece>, f64);

/// Where line `i` (`w` wide unstretched, with `spaces` spaces) starts in the box `r`, and its word
/// spacing. `paragraphs` holds each line's paragraph: a line takes its paragraph's alignment, and
/// every line of a justified paragraph but its last is stretched to the box's width.
fn placed(t: &AddedText, r: [f64; 4], paragraphs: &[usize], i: usize, w: f64, spaces: usize, rtl: bool) -> (f64, f64) {
    let paragraph = paragraphs.get(i).copied().unwrap_or(0);
    let align = t.paragraph_align(paragraph);
    let ends_paragraph = paragraphs.get(i + 1) != Some(&paragraph);
    let tw = if align == Align::Justify && !ends_paragraph && spaces > 0 { ((r[2] - r[0]) - w).max(0.0) / spaces as f64 } else { 0.0 };
    let x = match align {
        // A right-to-left paragraph's unstretched last line ends at the right edge.
        Align::Justify if rtl => r[2] - w - tw * spaces as f64,
        Align::Left | Align::Justify => r[0],
        Align::Center => r[0] + ((r[2] - r[0]) - w) / 2.0,
        Align::Right => r[2] - w,
    };
    (x, tw)
}

/// Draw a text item that has Arabic, Hebrew or Thai in it. Each line is laid out in display order; their glyphs
/// come from Type 3 fonts drawn from the craft-fonts face's outlines (no font file is embedded),
/// with a ToUnicode map for copy and search. Other text uses the item's standard font.
fn draw_shaped(doc: &mut Document, t: &AddedText, taken: &Dict, fonts: &mut Dict, out: &mut Vec<u8>) -> Result<(), EditError> {
    let laid = wrapped(t)
        .into_iter()
        .map(|(line, rtl, paragraph)| shaped_layout(t, &line, rtl).map(|(pieces, w)| (line, rtl, paragraph, pieces, w)))
        .collect::<Result<Vec<_>, _>>()?;
    // The item's Korean faces (empty without Korean): the head face names the fonts, and each
    // character's outline comes from the first face that has it.
    let korean = korean_faces(t).unwrap_or_default();
    draw_laid_shaped(
        doc,
        t,
        &laid,
        taken,
        fonts,
        out,
        |script, id| match script {
            ShapedScript::Korean => korean_outline(&korean, id),
            script => shaped_glyph(script, id),
        },
        korean.first().map(|f| f.family),
    )
}

/// The outline of the Korean character `id` (a Unicode code point, see the layout's
/// `shape_korean`) from the first face in `faces` that has it.
fn korean_outline(faces: &[&CraftFont], id: u32) -> Result<GlyphOutline, GlyphError> {
    let ch = char::from_u32(id).ok_or(GlyphError::Missing)?;
    let face = faces.iter().find(|f| craft_has(f, ch)).ok_or(GlyphError::Missing)?;
    japanese_glyph_from(face, ch)
}

/// [`draw_shaped`] for lines that are already laid out. `outline` is the source of glyph outlines
/// for a script's face (the craft-fonts faces, or a stand-in in tests). `korean_family` is the
/// family of the item's head Korean face, which names the fonts of an all-Korean chunk.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_laid_shaped(
    doc: &mut Document,
    t: &AddedText,
    laid: &[LaidLine],
    taken: &Dict,
    fonts: &mut Dict,
    out: &mut Vec<u8>,
    outline: impl Fn(ShapedScript, u32) -> Result<GlyphOutline, GlyphError>,
    korean_family: Option<&'static str>,
) -> Result<(), EditError> {
    let r = text_rect(t);
    let latin = font_name(t.family.base_font(t.bold, t.italic));
    let map_err = |e: GlyphError, c: &ShapedCluster| match c.script {
        ShapedScript::Arabic => arabic_error(e, &c.text),
        ShapedScript::Hebrew => hebrew_error(e, &c.text),
        ShapedScript::Thai => thai_error(e, &c.text),
        ShapedScript::Korean => korean_error(e, &c.text),
    };
    let (ko, rest): (Vec<&ShapedCluster>, Vec<&ShapedCluster>) =
        shaped_text::clusters(laid.iter().map(|(_, _, _, pieces, _)| pieces.as_slice())).partition(|c| c.script == ShapedScript::Korean);
    let ko_shaped = ShapedFonts::new(ko, &outline, taken, &mut *fonts, "PCKo", korean_family, &mut |o| {
        Object::Ref(doc.add(o))
    })
    .map_err(|(e, c)| map_err(e, c))?;
    let shaped = ShapedFonts::new(rest, &outline, taken, &mut *fonts, "PCAr", None, &mut |o| {
        Object::Ref(doc.add(o))
    })
    .map_err(|(e, c)| map_err(e, c))?;
    out.extend(format!("BT {}\n", t.fill_op()).bytes());
    if t.char_spacing != 0.0 {
        out.extend(format!("{} Tc\n", n(t.char_spacing)).bytes());
    }
    let paragraphs: Vec<usize> = laid.iter().map(|(_, _, p, ..)| *p).collect();
    for (i, (line, rtl, _, pieces, w)) in laid.iter().enumerate() {
        // Shaped glyphs are painted in visual order. Keep the original line's typing order
        // explicitly, including marks, ligatures, digits and mixed-script punctuation.
        let logical: String = line.encode_utf16().map(|u| format!("{u:04X}")).collect();
        out.extend(format!("/Span <</ActualText <FEFF{logical}>>> BDC\n").bytes());
        let spaces = line.matches(' ').count();
        let (x, tw) = placed(t, r, &paragraphs, i, *w, spaces, *rtl);
        let y = r[3] - (i as f64 * 1.2 + 0.95) * t.size;
        let mut pen = x;
        let mut current_ko = None;
        let mut current_ar = None;
        for piece in pieces {
            match piece {
                Piece::Latin(s) => {
                    out.extend(format!("/{latin} {} Tf {} Tw 1 0 0 1 {} {} Tm ", n(t.size), n(tw), n(pen), n(y)).bytes());
                    out.extend(literal(&win_ansi(s)));
                    out.extend_from_slice(b" Tj\n");
                    current_ko = None;
                    current_ar = None;
                    pen += latin_width(t, s) + tw * s.matches(' ').count() as f64;
                }
                Piece::Shaped(clusters) => {
                    pen = ko_shaped.show(out, clusters, pen, y, t.size, t.char_spacing, tw, &mut current_ko);
                    pen = shaped.show(out, clusters, pen, y, t.size, t.char_spacing, tw, &mut current_ar);
                }
            }
        }
        out.extend_from_slice(b"EMC\n");
    }
    out.extend_from_slice(b"ET\n");
    Ok(())
}

/// A word the recognizer found, in PDF user space: the bottom-left corner of its box
/// (`origin`) and the vectors along its bottom (`across`, in reading direction) and left
/// (`up`) edges. Mirrors the recognizer's own word type, kept independent of that crate.
#[derive(Clone, Debug, PartialEq)]
pub struct OcrWord {
    pub text: String,
    pub origin: [f64; 2],
    pub across: [f64; 2],
    pub up: [f64; 2],
}

/// Where a Hangul syllable's ink stands against its em, as the invisible OCR layer reads it:
/// the Korean faces draw the block from about this far above the baseline down to this far
/// below, so a recognized box of height `h` takes a text matrix of `h / (KO_TOP - KO_BOTTOM)`.
/// The Latin layer's `text_layer` counterpart uses Helvetica's cap height and descender.
const KO_TOP: f64 = 0.93;
const KO_BOTTOM: f64 = -0.21;

/// The invisible text over a recognized Korean page: each word is drawn from the craft-fonts
/// Korean faces through the same Type 3 pipeline the editor uses when it writes Hangul into a
/// document, so the glyphs travel with the faces' ToUnicode map and the words come back on
/// copy, search and selection. Everything is painted in rendering mode 3 inside `/OCR` marked
/// content, exactly like the Latin layer, and the page's picture is left untouched. A word the
/// Korean faces cannot draw is left out of the layer rather than written as `?`.
///
/// `words` should be the ones the recognizer's `fits_layer_font` rejects — pure-Latin words
/// belong to the Helvetica layer. Does nothing when there is nothing it can draw.
pub fn ocr_korean_layer(doc: &mut Document, page: usize, words: &[OcrWord]) -> Result<(), EditError> {
    let all = page_list(doc);
    check(&[page], all.len())?;
    let p = &all[page];
    check_on_top(doc, p)?;
    let faces = crate::text::korean_face_list(Family::Helvetica, false)?;

    // One cluster per character: Hangul composes without shaping (the visible path does the
    // same, see `shape_korean`), and a character no Korean face holds is skipped here, so it
    // is skipped from its word rather than turned into a `?`.
    let mut pieces: Vec<Vec<ShapedCluster>> = Vec::with_capacity(words.len());
    for w in words {
        let mut pw = Vec::new();
        for c in w.text.chars() {
            if c.is_whitespace() {
                continue;
            }
            let Some(advance) = faces.iter().find_map(|f| craft_advance(f, c)) else { continue };
            pw.push(ShapedCluster { script: ShapedScript::Korean, glyphs: vec![(u32::from(c), [0.0, 0.0])], advance, text: c.to_string() });
        }
        pieces.push(pw);
    }
    if pieces.iter().all(|pw| pw.is_empty()) {
        return Ok(());
    }

    // The page's resources, read before any object is created.
    let pres = p.dict.get(b"Resources").map(|r| doc.resolve(r)).and_then(|r| r.as_dict().cloned()).unwrap_or_default();
    let taken = pres.get(b"Font").map(|f| doc.resolve(f)).and_then(|f| f.as_dict().cloned()).unwrap_or_default();

    // One glyph per distinct cluster in Type 3 fonts named like the visible path's, so
    // `own_fonts` cleanup stays uniform across Korean content. Every outline is read before
    // anything is written: a missing face fails the whole edit rather than leaving half a
    // layer on the page (never-crash).
    let mut ocr_fonts = Dict::new();
    let ko = ShapedFonts::new(
        pieces.iter().flatten(),
        |_, id| korean_outline(&faces, id),
        &taken,
        &mut ocr_fonts,
        "PCKo",
        faces.first().map(|f| f.family),
        &mut |o| Object::Ref(doc.add(o)),
    )
    .map_err(|(e, c)| korean_error(e, &c.text))?;

    // Content: every word's matrix stretches its run codes into the box the recognizer
    // measured, invisibly (`3 Tr`), as one text block inside the `/OCR` marked content.
    let mut out: Vec<u8> = b"/OCR BMC\nq\nBT\n3 Tr\n".to_vec();
    let mut drawn = false;
    for (w, pw) in words.iter().zip(pieces.iter()) {
        if pw.is_empty() {
            continue;
        }
        let natural: f64 = pw.iter().map(|cl| cl.advance).sum();
        let (ux, uy) = (w.up[0], w.up[1]);
        let up_len = ux.hypot(uy);
        let across = [w.across[0], w.across[1]];
        if !natural.is_finite() || natural <= 0.0 || !up_len.is_finite() || up_len <= 0.0 {
            continue;
        }
        let em = [ux / (KO_TOP - KO_BOTTOM), uy / (KO_TOP - KO_BOTTOM)];
        let o = [w.origin[0] - em[0] * KO_BOTTOM, w.origin[1] - em[1] * KO_BOTTOM];
        // Codes grouped into runs by the font they live in; one Tm per run.
        let mut runs: Vec<(&str, String)> = Vec::new();
        for cl in pw {
            let Some((name, code)) = ko.code(cl) else { continue };
            match runs.last_mut() {
                Some((cur, hex)) if *cur == name => {
                    let _ = write!(hex, "{code:02X}");
                }
                _ => runs.push((name, format!("{code:02X}"))),
            }
        }
        if runs.is_empty() {
            continue;
        }
        let ax = [across[0] / natural, across[1] / natural];
        for (name, hex) in runs {
            out.extend(format!("/{name} 1 Tf\n{} {} {} {} {} {} Tm\n<{hex}> Tj\n", n(ax[0]), n(ax[1]), n(em[0]), n(em[1]), n(o[0]), n(o[1])).bytes());
            drawn = true;
        }
    }
    out.extend_from_slice(b"ET\nQ\nEMC\n");
    if !drawn {
        // Nothing fit the boxes: leave the page alone. (The chunk fonts above stay as
        // unreferenced objects, which a full save drops; a page never shows a font it does
        // not name.)
        return Ok(());
    }

    let mut res_fonts = taken;
    for (name, font) in ocr_fonts.iter() {
        res_fonts.set(name.clone(), font.clone());
    }
    let mut pres = pres;
    pres.set(b"Font".to_vec(), Object::Dict(res_fonts));
    doc.update_dict(p.obj, |d| d.set(b"Resources".to_vec(), Object::Dict(pres)))?;
    let mut list = page_list(doc);
    let p = list.swap_remove(page);
    place_tagged(doc, &p, "OCR", out, false)?;
    Ok(())
}

/// Content and resources for an item; `view` maps display space to user space. `taken` holds the
/// page's font names. `existing`: the item is being replaced, so a build without the Arabic or
/// Hebrew face keeps drawing that text as `?`, as before it was supported, rather than refusing to
/// move it.
pub(super) fn draw(doc: &mut Document, c: &Content, view: [f64; 6], taken: &Dict, existing: bool) -> Result<(Vec<u8>, Dict), EditError> {
    let mut res = Dict::new();
    let mut out = format!(
        "q {} {} {} {} {} {} cm\n",
        matrix_number(view[0]),
        matrix_number(view[1]),
        matrix_number(view[2]),
        matrix_number(view[3]),
        matrix_number(view[4]),
        matrix_number(view[5])
    )
    .into_bytes();
    match c {
        Content::Text(t) => {
            let base = t.family.base_font(t.bold, t.italic);
            let name = font_name(base);
            let mut font = Dict::new();
            font.set(b"Type".to_vec(), Object::name("Font"));
            font.set(b"Subtype".to_vec(), Object::name("Type1"));
            font.set(b"BaseFont".to_vec(), Object::name(base));
            if t.family != Family::Courier || !t.text.is_ascii() {
                font.set(b"Encoding".to_vec(), Object::name("WinAnsiEncoding"));
            }
            let mut fonts = Dict::new();
            fonts.set(name.clone().into_bytes(), Object::Dict(font));
            let face_or_new = |has: fn(char) -> bool, face: bool| t.text.chars().any(has) && (face || !existing);
            if t.text.chars().any(is_thai)
                || face_or_new(is_arabic, pdfcraft_fonts::document_arabic_font().is_some())
                || face_or_new(is_hebrew, pdfcraft_fonts::document_hebrew_font(false).is_some())
                || face_or_new(is_korean, korean_faces(t).is_ok())
            {
                draw_shaped(doc, t, taken, &mut fonts, &mut out)?;
                res.set(b"Font".to_vec(), Object::Dict(fonts));
                out.extend_from_slice(b"Q\n");
                return Ok((out, res));
            }
            res.set(b"Font".to_vec(), Object::Dict(fonts));
            let r = text_rect(t);
            out.extend(format!("BT /{name} {} Tf {}\n", n(t.size), t.fill_op()).bytes());
            if t.char_spacing != 0.0 {
                out.extend(format!("{} Tc\n", n(t.char_spacing)).bytes());
            }
            let all = wrapped(t);
            let paragraphs: Vec<usize> = all.iter().map(|(_, _, p)| *p).collect();
            for (i, (line, ..)) in all.iter().enumerate() {
                let w = latin_width(t, line);
                // Baseline: 0.8 em below the line top.
                let y = r[3] - (i as f64 * 1.2 + 0.95) * t.size;
                // Justified: word spacing makes every line of a paragraph but its last fill the box.
                let (x, tw) = placed(t, r, &paragraphs, i, w, line.matches(' ').count(), false);
                out.extend(format!("1 0 0 1 {} {} Tm {} Tw ", n(x), n(y), n(tw)).bytes());
                out.extend(literal(&win_ansi(line)));
                out.extend_from_slice(b" Tj\n");
            }
            out.extend_from_slice(b"ET\n");
        }
        Content::Image(i) => {
            if !matches!(&*doc.get(i.image), Object::Stream(s) if s.dict.name(b"Subtype") == Some(b"Image")) {
                return Err(EditError::Invalid("not an image".into()));
            }
            let r = norm(i.rect);
            let name = format!("PCImg{}", i.image.num);
            let mut xo = Dict::new();
            xo.set(name.clone().into_bytes(), Object::Ref(i.image));
            res.set(b"XObject".to_vec(), Object::Dict(xo));
            let [a, b, c, d, e, f] = i.matrix().0;
            // Clip to the box: the cropped-away parts stay hidden.
            out.extend(
                format!(
                    "{} {} {} {} re W n {} {} {} {} {} {} cm /{name} Do\n",
                    n(r[0]),
                    n(r[1]),
                    n(r[2] - r[0]),
                    n(r[3] - r[1]),
                    n(a),
                    n(b),
                    n(c),
                    n(d),
                    n(e),
                    n(f)
                )
                .bytes(),
            );
        }
    }
    out.extend_from_slice(b"Q\n");
    Ok((out, res))
}

pub(super) fn params(c: &Content) -> Dict {
    let mut d = Dict::new();
    let arr = |v: &[f64]| Object::Array(v.iter().map(|x| Object::Real(*x)).collect());
    match c {
        Content::Text(t) => {
            d.set(b"Kind".to_vec(), Object::name("Text"));
            d.set(b"Text".to_vec(), PdfString::text(&t.text));
            d.set(b"Font".to_vec(), Object::name(t.family.base_font(t.bold, t.italic)));
            d.set(b"Size".to_vec(), Object::Real(t.size));
            d.set(b"Color".to_vec(), arr(&t.color));
            // The ink beside its RGB look, which a reader that knows no ink keeps showing.
            match t.painted_ink() {
                Some(Ink::Gray(g)) => d.set(b"Gray".to_vec(), Object::Real(g)),
                Some(Ink::Cmyk(c)) => d.set(b"CMYK".to_vec(), arr(&c)),
                None => {}
            }
            d.set(b"Align".to_vec(), Object::Int(align_code(t.align)));
            // Each paragraph's own alignment, or null where it follows /Align.
            let aligns = crate::paragraph_align::tidy(t.aligns.clone(), t.align);
            if !aligns.is_empty() {
                d.set(b"Aligns".to_vec(), Object::Array(aligns.iter().map(|a| a.map_or(Object::Null, |a| Object::Int(align_code(a)))).collect()));
            }
            d.set(b"Rect".to_vec(), arr(&text_rect(t)));
            if t.char_spacing != 0.0 {
                d.set(b"CharSpacing".to_vec(), Object::Real(t.char_spacing));
            }
        }
        Content::Image(i) => {
            d.set(b"Kind".to_vec(), Object::name("Image"));
            d.set(b"Image".to_vec(), Object::Ref(i.image));
            d.set(b"Rect".to_vec(), arr(&norm(i.rect)));
            if i.rotation % 4 != 0 {
                d.set(b"Rotate".to_vec(), Object::Int(i64::from(i.rotation % 4) * 90));
            }
            if i.flip_h {
                d.set(b"FlipH".to_vec(), Object::Bool(true));
            }
            if i.flip_v {
                d.set(b"FlipV".to_vec(), Object::Bool(true));
            }
            if i.crop != [0.0; 4] {
                d.set(b"Crop".to_vec(), arr(&i.crop));
            }
        }
    }
    d
}

/// Existing Thai text uses absolute shaped glyph positions. Reuse Add Text's bounded outline
/// writer instead of trying to embed the matching craft-fonts variable program as a static face.
pub(crate) fn thai_glyph_resources(
    doc: &mut Document,
    fonts: &mut Dict,
    shaped: &pdfcraft_fonts::source_shaping::ShapedRun,
) -> Result<Vec<(String, Vec<u8>)>, EditError> {
    if shaped.glyphs.len() > 65_534 {
        return Err(EditError::Invalid("shaped row exceeds its geometry budget".into()));
    }
    let mut outlines = std::collections::HashMap::new();
    let mut clusters = Vec::new();
    let mut index = std::collections::HashMap::new();
    let mut indices = Vec::new();
    for glyph in &shaped.glyphs {
        let id = glyph.id.to_u32();
        let key = (ShapedScript::Thai, id);
        if let std::collections::hash_map::Entry::Vacant(entry) = outlines.entry(key) {
            entry.insert(shaped_glyph(ShapedScript::Thai, id).map_err(|e| thai_error(e, &glyph.text))?);
        }
        let at = *index.entry((id, glyph.text.as_str())).or_insert_with(|| {
            let at = clusters.len();
            let width = outlines.get(&key).map_or(0.0, |g| g.width);
            clusters.push(pdfcraft_fonts::ShapedCluster {
                script: ShapedScript::Thai,
                glyphs: vec![(id, [0.0, 0.0])],
                advance: width,
                text: glyph.text.clone(),
            });
            at
        });
        indices.push(at);
    }
    // Validate all outlines before adding resources. Type 3 character codes are single bytes;
    // shard long rows and return the appropriate resource/code for every positioned glyph.
    let mut names = Vec::new();
    for chunk in clusters.chunks(shaped_text::GLYPHS_PER_FONT) {
        let refs: Vec<_> = chunk.iter().collect();
        let font = shaped_text::add_type3_font(doc, &refs, &outlines);
        doc.update_dict(font, |f| f.set(b"Name".to_vec(), Object::name("PCThai")))?;
        let mut name = format!("PCThai{}", font.num);
        while fonts.contains(name.as_bytes()) {
            name.push('_');
        }
        fonts.set(name.as_bytes().to_vec(), Object::Ref(font));
        names.push(name);
    }
    indices
        .into_iter()
        .map(|at| {
            let name = names.get(at / shaped_text::GLYPHS_PER_FONT).ok_or_else(|| EditError::Invalid("Thai font resource is missing".into()))?;
            let code = u8::try_from(at % shaped_text::GLYPHS_PER_FONT + 1).map_err(|_| EditError::Invalid("Thai glyph limit".into()))?;
            Ok((name.clone(), vec![code]))
        })
        .collect()
}
