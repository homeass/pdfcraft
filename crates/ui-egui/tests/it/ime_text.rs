//! Reported bug 2: in the document text editors — the paragraph box (Edit text & images) and the
//! Add text box — a second input-method syllable was said to replace the first one, leaving the
//! caret before it. Both editors are plain `egui::TextEdit`s, so this drives the exact event
//! stream egui-winit forwards from an input method: a pre-edit for one frame, then the committed
//! syllable. Nothing here is platform code, so it settles whether the defect is in the app/egui
//! path or only in what a physical keyboard's input method does before these events arrive.

use egui::{ImeEvent, Key, Modifiers, Pos2};
use egui_kittest::Harness;
use pdfcraft_ui_egui::PdfCraftApp;

use crate::support;

/// A one-page PDF whose page shows "Page 1" in Helvetica 24 at (20, 150) on a 200 × 300 pt page.
fn fixture() -> Vec<u8> {
    let objs: [Vec<u8>; 5] = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [4 0 R] /Count 1 /MediaBox [0 0 200 300] >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 5 0 R /Resources << /Font << /F1 3 0 R >> >> >>".to_vec(),
        {
            let body = "BT /F1 24 Tf 20 150 Td (Page 1) Tj ET";
            format!("<< /Length {} >>\nstream\n{body}\nendstream", body.len()).into_bytes()
        },
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objs.len() + 1).as_bytes());
    out
}

fn harness() -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.open_bytes("text.pdf", None, fixture()).unwrap();
        app.set_option("zoom", "150").unwrap();
        app
    });
    support::settle(&mut h);
    h
}

/// A point `fx` across and `fy` down the first page's on-screen rectangle.
fn page_point(h: &Harness<'static, PdfCraftApp>, fx: f32, fy: f32) -> Pos2 {
    let r = h.state().views[0].page_screen_rect(0).expect("page 1 is on screen");
    r.min + egui::vec2(r.width() * fx, r.height() * fy)
}

/// A deliberate click: hover, press, release, each in its own frame.
fn click(h: &mut Harness<'static, PdfCraftApp>, p: Pos2) {
    h.hover_at(p);
    h.run_steps(1);
    h.drag_at(p);
    h.run_steps(1);
    h.drop_at(p);
    h.run_steps(3);
}

/// The focused widget, checked to be the document text editor `what` and to own the IME context.
fn focused_text_edit(h: &Harness<'static, PdfCraftApp>, what: &str) -> egui::Id {
    let id = h.ctx.memory(|m| m.focused()).unwrap_or_else(|| panic!("{what}: no widget has the keyboard"));
    assert!(egui::TextEdit::load_state(&h.ctx, id).is_some(), "{what}: the focused widget is not a text editor");
    let ime = h.output().platform_output.ime.as_ref().unwrap_or_else(|| panic!("{what}: the focused editor advertises no input method"));
    assert_eq!(ime.owner, id, "{what}: the input method context belongs to another widget");
    id
}

/// The primary caret of the text editor `id`, in characters.
fn caret(h: &Harness<'static, PdfCraftApp>, id: egui::Id) -> usize {
    let state = egui::TextEdit::load_state(&h.ctx, id).unwrap_or_else(|| panic!("no text editor state"));
    state.cursor.char_range().unwrap_or_else(|| panic!("the text editor has no caret")).primary.index.0
}

/// One syllable as an input method delivers it: the pre-edit for a frame, then the commit for a
/// frame. Neither frame, nor an idle one after them, may ask egui-winit to interrupt the
/// composition — an interruption switches the input method off and on, which drops the commit.
fn compose(h: &mut Harness<'static, PdfCraftApp>, syllable: &str, phase: &str) -> Vec<bool> {
    let chars = syllable.chars().count();
    let mut interruptions = Vec::new();
    h.event(egui::Event::Ime(ImeEvent::Preedit { text: syllable.into(), active_range_chars: Some(0..chars) }));
    h.step();
    interruptions.push(interrupted(h, &format!("{phase} pre-edit")));
    h.event(egui::Event::Ime(ImeEvent::Commit(syllable.into())));
    h.step();
    interruptions.push(interrupted(h, &format!("{phase} commit")));
    h.step();
    interruptions.push(interrupted(h, &format!("{phase} idle")));
    interruptions
}

fn interrupted(h: &Harness<'static, PdfCraftApp>, phase: &str) -> bool {
    let ime = h.output().platform_output.ime.as_ref().unwrap_or_else(|| panic!("{phase}: no input method context"));
    eprintln!("{phase}: owner={:?}, should_interrupt_composition={}", ime.owner, ime.should_interrupt_composition);
    ime.should_interrupt_composition
}

/// The paragraph box: two committed syllables append and the caret stays after them, and a Latin
/// key typed afterwards lands after the composed text instead of before it.
#[test]
fn the_paragraph_editor_appends_input_method_commits_at_the_caret() {
    let test = "the_paragraph_editor_appends_input_method_commits_at_the_caret";
    if support::skip_without_glyphs(test, "en", "한가") {
        return;
    }
    let mut h = harness();
    assert!(h.state_mut().execute("edit.edit_text"));
    h.run_steps(2);
    let point = page_point(&h, 0.2, 0.473);
    click(&mut h, point);
    h.run_steps(3);
    let opened = h.state().views[0].line_editor.clone().expect("the click opens the page's paragraph");
    assert_eq!(opened.text, "Page 1", "the click opened the page's own paragraph");
    let id = focused_text_edit(&h, "the paragraph editor");
    // An empty paragraph makes every committed syllable the whole draft, so the caret index is
    // the number of committed characters.
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.key_press(Key::Backspace);
    h.run_steps(2);
    assert_eq!(h.state().views[0].line_editor.as_ref().map(|e| e.text.as_str()), Some(""), "the draft is cleared");
    assert_eq!(caret(&h, id), 0, "the caret is at the start of the empty paragraph");

    let first = compose(&mut h, "한", "first syllable");
    assert_eq!(h.state().views[0].line_editor.as_ref().map(|e| e.text.as_str()), Some("한"), "the committed syllable is in the draft");
    assert_eq!(caret(&h, id), 1, "the caret follows the committed syllable");

    let second = compose(&mut h, "가", "second syllable");
    assert_eq!(
        h.state().views[0].line_editor.as_ref().map(|e| e.text.as_str()),
        Some("한가"),
        "the second commit appends; it must not replace the first"
    );
    assert_eq!(caret(&h, id), 2, "the caret follows the second committed syllable");

    h.event(egui::Event::Text("a".into()));
    h.run_steps(2);
    assert_eq!(h.state().views[0].line_editor.as_ref().map(|e| e.text.as_str()), Some("한가a"), "a Latin key after a commit lands at the end");
    assert_eq!(caret(&h, id), 3);

    let interruptions: Vec<bool> = first.into_iter().chain(second).collect();
    assert!(interruptions.iter().all(|interrupt| !interrupt), "the paragraph editor interrupted the composition: {interruptions:?}");
}

/// The Add text box: the same commits, the same caret, the same Latin key afterwards.
#[test]
fn the_added_text_editor_appends_input_method_commits_at_the_caret() {
    let test = "the_added_text_editor_appends_input_method_commits_at_the_caret";
    if support::skip_without_glyphs(test, "en", "한가") {
        return;
    }
    let mut h = harness();
    assert!(h.state_mut().execute("edit.text"));
    h.run_steps(2);
    let point = page_point(&h, 0.3, 0.15);
    click(&mut h, point);
    h.run_steps(3);
    let draft = h.state().views[0].content.draft.as_ref().expect("the click starts an added-text box");
    assert!(draft.text.is_empty(), "a new added-text box starts empty");
    let id = focused_text_edit(&h, "the Add text editor");
    assert_eq!(caret(&h, id), 0);

    let first = compose(&mut h, "한", "first syllable");
    assert_eq!(h.state().views[0].content.draft.as_ref().map(|d| d.text.as_str()), Some("한"), "the committed syllable is in the box");
    assert_eq!(caret(&h, id), 1, "the caret follows the committed syllable");

    let second = compose(&mut h, "가", "second syllable");
    assert_eq!(
        h.state().views[0].content.draft.as_ref().map(|d| d.text.as_str()),
        Some("한가"),
        "the second commit appends; it must not replace the first"
    );
    assert_eq!(caret(&h, id), 2, "the caret follows the second committed syllable");

    h.event(egui::Event::Text("a".into()));
    h.run_steps(2);
    assert_eq!(h.state().views[0].content.draft.as_ref().map(|d| d.text.as_str()), Some("한가a"), "a Latin key after a commit lands at the end");
    assert_eq!(caret(&h, id), 3);

    let interruptions: Vec<bool> = first.into_iter().chain(second).collect();
    assert!(interruptions.iter().all(|interrupt| !interrupt), "the Add text editor interrupted the composition: {interruptions:?}");
}
