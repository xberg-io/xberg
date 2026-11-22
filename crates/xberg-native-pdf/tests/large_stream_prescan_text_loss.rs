//! End-to-end: crossing the content-stream size at which span extraction
//! switches to pre-scanning for text regions must not change the text a page
//! yields.
//!
//! Above that size the extractor locates text regions and replays each one
//! with a reconstructed graphics state instead of parsing the stream front to
//! back. Both fixtures here are shapes taken from real CAD/plan sheets, where
//! the stream is one long line of `cm`-scaled marked-content blocks:
//!
//!   * a marked-content block that scales the CTM between text objects — the
//!     region is extended back over its `BDC`, so the operators between the
//!     `BDC` and the `BT` are replayed on top of the injected state and the
//!     scale applies twice, putting the text far outside the MediaBox where
//!     the off-page filter deletes it;
//!   * a sheet whose text lives in re-used Form XObjects, where `Do`
//!     invocations outnumber page-level `BT` blocks.
//!
//! Each is asserted against the same page built under the threshold, so the
//! fixtures pin "the size of the surrounding linework must not matter" rather
//! than any particular extraction quality.

use xberg_native_pdf::PdfDocument;

const W: u32 = 2592;
const H: u32 = 1728;

/// `pad` bytes of vector linework, written without newlines exactly as CAD
/// exporters emit it.
fn linework(pad: usize) -> String {
    let mut s = String::new();
    let mut n = 0usize;
    while s.len() < pad {
        let x = (n * 7) % W as usize;
        let y = (n * 13) % H as usize;
        s.push_str(&format!("{x} {y} m {} {} l {} {} l S ", x + 5, y + 9, x + 11, y + 3));
        n += 1;
    }
    s
}

/// Assemble a one-page PDF from a content stream and an optional XObject
/// resource entry, with `forms` as extra stream objects starting at id 6.
fn one_page_pdf(content: &str, forms: &[(String, String)]) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let last_id = 5 + forms.len();
    let mut off = vec![0usize; last_id];
    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id - 1] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };
    let stream_obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, dict: &str, s: &str| {
        off[id - 1] = buf.len();
        buf.extend_from_slice(
            format!(
                "{id} 0 obj\n<< {dict} /Length {} >>\nstream\n{s}\nendstream\nendobj\n",
                s.len()
            )
            .as_bytes(),
        );
    };

    let xobject_res = if forms.is_empty() {
        String::new()
    } else {
        let entries: Vec<String> = forms
            .iter()
            .enumerate()
            .map(|(i, (name, _))| format!("/{name} {} 0 R", 6 + i))
            .collect();
        format!("/XObject << {} >>", entries.join(" "))
    };

    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        &format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {W} {H}] \
             /Resources << /Font << /F1 5 0 R >> {xobject_res} >> /Contents 4 0 R >>"
        ),
    );
    stream_obj(&mut buf, &mut off, 4, "", content);
    obj(
        &mut buf,
        &mut off,
        5,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    );
    for (i, (_, body)) in forms.iter().enumerate() {
        stream_obj(
            &mut buf,
            &mut off,
            6 + i,
            "/Type /XObject /Subtype /Form /BBox [0 0 400 40] \
             /Resources << /Font << /F1 5 0 R >> >>",
            body,
        );
    }

    let xref = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", last_id + 1).as_bytes());
    for o in off.iter() {
        buf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            last_id + 1
        )
        .as_bytes(),
    );
    buf
}

/// A marked-content block that scales the CTM between text objects: the shape
/// an AutoCAD sheet emits, drawing at `0.12` inside an `8.3333333` block.
fn scaled_marked_content_sheet(pad: usize) -> Vec<u8> {
    let mut c = linework(pad);
    c.push_str(
        "BT /F1 58.66 Tf 0 0.12 -0.12 0 663.36 978 Tm \
         (SCHEDULE OF FINISHES) Tj (GENERAL NOTES) Tj ET ",
    );
    c.push_str("/OC /MC7 BDC ");
    c.push_str("8.3333333 0 0 8.3333333 0 0 cm ");
    c.push_str("BT (A) Tj ET ");
    c.push_str("BT 0 0.12 -0.12 0 663.36 1077 Tm (TITLE BLOCK) Tj ET ");
    one_page_pdf(&c, &[])
}

/// A sheet whose text lives in re-used Form XObjects, invoked far more often
/// than the page stream begins a text object. `n_page_bt` page-level text
/// blocks stand in for a title block drawn directly on the sheet.
fn form_xobject_sheet_with_page_text(pad: usize, n_forms: usize, n_page_bt: usize) -> Vec<u8> {
    let forms: Vec<(String, String)> = (0..n_forms)
        .map(|i| {
            (
                format!("Fm{i}"),
                format!("BT /F1 12 Tf 10 10 Td (NOTE {i:03} GENERAL NOTES) Tj ET"),
            )
        })
        .collect();
    let mut c = linework(pad);
    for i in 0..n_page_bt {
        c.push_str(&format!(
            "BT /F1 14 Tf 100 {} Td (PAGE LEVEL TITLE {i:02}) Tj ET ",
            1600 - i * 20
        ));
    }
    for i in 0..n_forms {
        let x = 100 + (i % 5) * 450;
        let y = 1500 - (i / 5) * 60;
        c.push_str(&format!("q 1 0 0 1 {x} {y} cm /Fm{i} Do Q "));
    }
    one_page_pdf(&c, &forms)
}

fn form_xobject_sheet(pad: usize, n_forms: usize) -> Vec<u8> {
    form_xobject_sheet_with_page_text(pad, n_forms, 0)
}

fn text_of(pdf: Vec<u8>) -> String {
    PdfDocument::from_bytes(pdf)
        .expect("parse")
        .extract_text(0)
        .expect("extract_text")
}

fn words(s: &str) -> Vec<&str> {
    s.split_whitespace().collect()
}

#[test]
fn scaled_marked_content_survives_a_large_stream() {
    let small = text_of(scaled_marked_content_sheet(2_000));
    let big = text_of(scaled_marked_content_sheet(300_000));

    // The last text object is drawn after a bare `cm` that scales it off the
    // sheet, so it is legitimately absent from both — the fixture pins the two
    // against each other, not against a hand-written expectation.
    assert!(
        small.contains("SCHEDULE OF FINISHES"),
        "control fixture is wrong: small stream already loses text, got {small:?}"
    );
    assert_eq!(
        words(&small),
        words(&big),
        "linework padding alone changed the extracted text"
    );
}

#[test]
fn form_xobject_text_survives_a_large_stream() {
    let small = text_of(form_xobject_sheet(1_000, 60));
    let big = text_of(form_xobject_sheet(300_000, 60));

    assert!(
        small.contains("NOTE 000") && small.contains("NOTE 059"),
        "control fixture is wrong: small stream already loses text"
    );
    assert_eq!(
        words(&small),
        words(&big),
        "linework padding alone changed the extracted text"
    );
}

#[test]
fn page_level_text_does_not_mask_lost_form_text() {
    // A title block drawn on the sheet alongside blocks that carry the
    // schedules: the page returns *some* text either way, so only comparing
    // the two sizes catches the blocks going missing.
    let small = text_of(form_xobject_sheet_with_page_text(1_000, 60, 5));
    let big = text_of(form_xobject_sheet_with_page_text(300_000, 60, 5));

    assert!(
        small.contains("PAGE LEVEL TITLE 00") && small.contains("NOTE 059"),
        "control fixture is wrong: small stream already loses text"
    );
    assert_eq!(
        words(&small),
        words(&big),
        "linework padding alone changed the extracted text"
    );
}

/// A sheet that *declares* a Form XObject it never draws.
///
/// CONTRIBUTING §4: this change broadens what the pre-scan retains — it
/// samples graphics state at the region start rather than the `BT`/`Do`, and
/// falls back to a full parse when `Do` invocations outnumber `BT` blocks.
/// Everything wrongly retained by a broadening change shows up as *more
/// output*, which reads as recovered content, so a fixture is needed in which
/// the broadened path would pick up something it must not.
///
/// The out-of-scope match here is a form present in `/XObject` resources with
/// no `Do` that invokes it. A conformant reader paints only what the content
/// stream draws (ISO 32000-1 §8.10.1: the form is painted *by* the `Do`
/// operator), so its text must never appear. A fallback that enumerated
/// resources instead of following the draw sequence would surface it — and
/// would look like a text-recovery improvement while doing so.
fn sheet_with_an_undrawn_form(pad: usize, n_drawn: usize) -> Vec<u8> {
    let mut forms: Vec<(String, String)> = (0..n_drawn)
        .map(|i| {
            (
                format!("Fm{i}"),
                format!("BT /F1 12 Tf 10 10 Td (NOTE {i:03} GENERAL NOTES) Tj ET"),
            )
        })
        .collect();
    // Declared in /XObject resources, never invoked by any `Do` below.
    forms.push((
        "FmGhost".to_string(),
        "BT /F1 12 Tf 10 10 Td (UNDRAWN GHOST) Tj ET".to_string(),
    ));

    let mut c = linework(pad);
    c.push_str("BT /F1 14 Tf 100 1600 Td (SHEET TITLE) Tj ET ");
    for i in 0..n_drawn {
        let x = 100 + (i % 5) * 450;
        let y = 1500 - (i / 5) * 60;
        c.push_str(&format!("q 1 0 0 1 {x} {y} cm /Fm{i} Do Q "));
    }
    one_page_pdf(&c, &forms)
}

#[test]
fn test_undrawn_form_is_not_collected_at_either_stream_size() {
    // 60 drawn forms against 1 page-level BT puts the Do:BT ratio well past
    // the 10:1 point, so this exercises the fallback this change introduces —
    // which is the broadened path §4 is asking about.
    let small = text_of(sheet_with_an_undrawn_form(2_000, 60));
    let big = text_of(sheet_with_an_undrawn_form(300_000, 60));

    // Control: the drawn content must be present, or the fixture proves nothing.
    assert!(
        small.contains("SHEET TITLE") && small.contains("NOTE 059"),
        "control fixture is wrong: drawn text missing below the threshold, got {small:?}"
    );
    assert!(
        big.contains("SHEET TITLE") && big.contains("NOTE 059"),
        "drawn text lost above the pre-scan threshold, got {big:?}"
    );

    // The §4 assertion, on BOTH sides of the threshold.
    assert!(
        !small.contains("UNDRAWN GHOST"),
        "text from a never-invoked form leaked below the threshold: {small:?}"
    );
    assert!(
        !big.contains("UNDRAWN GHOST"),
        "text from a never-invoked form leaked above the threshold — the pre-scan \
         fallback enumerated resources instead of following the draw sequence: {big:?}"
    );

    assert_eq!(
        words(&small),
        words(&big),
        "linework padding alone changed the extracted text"
    );
}

/// The low-`Do`:`BT` case: a sheet with a single drawn form.
///
/// The 10:1 fallback only fires above 50 `Do` invocations, so a sheet with
/// one form still goes through the pre-scan. A `Do` region ends at `tp + 2`,
/// which covers only the operator — the `/Name` operand has to come from the
/// region START. When no BDC/BMC precedes it the start is the `Do` itself, so
/// the region replayed a bare `Do` against an empty operand stack, the form
/// was never invoked, and every glyph it drew was lost.
#[test]
fn test_single_drawn_form_survives_a_large_stream() {
    let small = text_of(sheet_with_an_undrawn_form(2_000, 1));
    let big = text_of(sheet_with_an_undrawn_form(300_000, 1));

    assert!(
        small.contains("SHEET TITLE") && small.contains("NOTE 000"),
        "control fixture is wrong: text missing below the threshold, got {small:?}"
    );
    assert!(
        big.contains("NOTE 000"),
        "single-form text lost above the pre-scan threshold — the Do region \
         excluded its /Name operand: {big:?}"
    );
    assert!(!big.contains("UNDRAWN GHOST"), "undrawn form leaked: {big:?}");
    assert_eq!(
        words(&small),
        words(&big),
        "linework padding alone changed the extracted text"
    );
}

/// A marked-content block whose `BDC`..`BT` window carries a `Q` with no
/// matching `q`. Plan sheets emit this when a layer's clip is released
/// mid-block: the enclosing save was opened further up the stream, or never,
/// and the restore lands inside the marked-content run.
///
/// The scale is set at the top level and is NOT re-established after the `Q`,
/// so the graphics state the text needs exists only in the reconstruction.
/// Extending the region back over the `BDC` would replay that unmatched `Q`
/// inside the SaveState/RestoreState wrapper, popping the *injected* save and
/// discarding the CTM — the text then draws at 1:1 instead of 0.12 and lands
/// outside the MediaBox, where the off-page filter deletes it. The region must
/// start at the `BT` instead, giving up the marked-content context rather than
/// the geometry.
fn unbalanced_restore_sheet(pad: usize) -> Vec<u8> {
    let mut c = linework(pad);
    // Sheet scale, set at the top level with no enclosing save.
    c.push_str("0.12 0 0 0.12 0 0 cm ");
    c.push_str("/OC /MC7 BDC ");
    // Clip released inside the block: BDC..BT now holds an unmatched `Q`.
    c.push_str("Q ");
    // Placed so it is on-page under the 0.12 sheet scale and far off-page
    // without it, which is what makes the two paths distinguishable at all.
    c.push_str(
        "BT /F1 58.66 Tf 1 0 0 1 10000 10000 Tm \
         (RELEASED CLIP NOTES) Tj ET ",
    );
    c.push_str("EMC ");
    one_page_pdf(&c, &[])
}

#[test]
fn text_after_an_unbalanced_restore_survives_a_large_stream() {
    let small = text_of(unbalanced_restore_sheet(1_000));
    let big = text_of(unbalanced_restore_sheet(300_000));
    // Control: the shape must extract below the pre-scan threshold, or the
    // comparison below would pass on two identical failures — which is how the
    // 60-form fixtures in this file passed while bypassing the pre-scan.
    assert!(
        small.contains("RELEASED CLIP NOTES"),
        "control failed: the sub-threshold page did not yield the text at all, so \
         this fixture cannot say anything about the pre-scan path (got {small:?})"
    );
    assert_eq!(
        words(&small),
        words(&big),
        "linework padding alone changed the extracted text"
    );
}
