//! A widget's normal appearance may be a **subdictionary of appearance
//! states**, and `/AS` names the one to draw.
//!
//! ISO 32000-1:2008 §12.5.5: "the appearance dictionary's /N, /R and /D
//! entries shall be … a subdictionary containing multiple appearance streams
//! … the /AS entry shall be present and shall specify which one is to be
//! used." That is the specification's own checkbox example.
//!
//! The renderer accepted `/N` only when it resolved to `Object::Stream`, so a
//! state subdictionary was silently skipped and every AcroForm checkbox and
//! radio button rendered blank — while `Annotation::appearance_state` was
//! parsed and read nowhere in the renderer.
//!
//! `Annotation::flags` was equally unread, so §12.5.3 Table 165's Hidden and
//! NoView annotations were painted.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{ImageFormat, RenderOptions, render_page};

/// A page with one widget annotation. `ap` is the `/AP` value, `extra` any
/// further annotation entries (e.g. `/AS`, `/F`).
fn widget_pdf(ap: &str, extra: &str) -> Vec<u8> {
    widget_pdf_with_off_dict(ap, extra, "")
}

/// As [`widget_pdf`], with `off_dict` added to the dictionary of the "off" appearance stream.
fn widget_pdf_with_off_dict(ap: &str, extra: &str, off_dict: &str) -> Vec<u8> {
    // Appearance streams: object 6 paints a black square, object 7 paints
    // nothing at all (the "off" state).
    let on = b"0 0 0 rg 0 0 20 20 re f\n";
    let off = b"\n";

    let mut buf: Vec<u8> = Vec::new();
    let mut off_tbl = vec![0usize; 9];
    let obj = |buf: &mut Vec<u8>, off_tbl: &mut Vec<usize>, id: usize, body: &str| {
        off_tbl[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };
    let form = |buf: &mut Vec<u8>, off_tbl: &mut Vec<usize>, id: usize, data: &[u8], dict: &str| {
        off_tbl[id] = buf.len();
        buf.extend_from_slice(
            format!(
                "{id} 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] {dict} \
                 /Length {} >>\nstream\n",
                data.len()
            )
            .as_bytes(),
        );
        buf.extend_from_slice(data);
        buf.extend_from_slice(b"\nendstream\nendobj\n");
    };

    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(&mut buf, &mut off_tbl, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off_tbl, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off_tbl,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
         /Annots [5 0 R] >>",
    );
    let content = b"\n";
    off_tbl[4] = buf.len();
    buf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    obj(
        &mut buf,
        &mut off_tbl,
        5,
        &format!(
            "<< /Type /Annot /Subtype /Widget /FT /Btn /Rect [10 10 30 30] \
             /AP {ap} {extra} >>"
        ),
    );
    form(&mut buf, &mut off_tbl, 6, on, "");
    form(&mut buf, &mut off_tbl, 7, off, off_dict);

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 8\n0000000000 65535 f \n");
    for offset in &off_tbl[1..=7] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref}\n%%EOF\n").as_bytes());
    buf
}

/// Whether anything darker than near-white was painted inside the widget's
/// rectangle.
fn widget_is_painted(pdf: Vec<u8>) -> bool {
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let mut opts = RenderOptions::default();
    opts.dpi = 72;
    opts.format = ImageFormat::RawRgba8;
    let img = render_page(&doc, 0, &opts).expect("render");
    let (w, h) = (img.width as usize, img.height as usize);
    // /Rect [10 10 30 30] in PDF space is y 70..90 from the raster top.
    for y in 72..88.min(h) {
        for x in 12..28.min(w) {
            let i = (y * w + x) * 4;
            if img.data[i] < 128 && img.data[i + 1] < 128 && img.data[i + 2] < 128 {
                return true;
            }
        }
    }
    false
}

/// The reported symptom: `/N` as a state subdictionary with `/AS` selecting
/// the on state must paint.
#[test]
fn checked_state_subdictionary_is_drawn() {
    assert!(
        widget_is_painted(widget_pdf("<< /N << /Off 7 0 R /Yes 6 0 R >> >>", "/AS /Yes")),
        "the /AS-selected appearance state was not drawn"
    );
}

/// Selecting the off state must paint nothing — the selection has to be real,
/// not "draw whichever stream we find".
#[test]
fn unchecked_state_subdictionary_draws_its_own_state() {
    assert!(
        !widget_is_painted(widget_pdf("<< /N << /Off 7 0 R /Yes 6 0 R >> >>", "/AS /Off")),
        "the off state should paint nothing"
    );
}

/// §12.5.5 blesses displaying nothing when the state cannot be resolved.
#[test]
fn test_state_subdictionary_without_as_draws_nothing() {
    assert!(!widget_is_painted(widget_pdf(
        "<< /N << /Off 7 0 R /Yes 6 0 R >> >>",
        ""
    )));
}

/// An `/AS` naming no member is the same case.
#[test]
fn test_as_naming_no_member_draws_nothing() {
    assert!(!widget_is_painted(widget_pdf(
        "<< /N << /Off 7 0 R /Yes 6 0 R >> >>",
        "/AS /Missing"
    )));
}

/// The plain-stream form must keep working — the common case for every
/// non-button widget.
#[test]
fn test_plain_appearance_stream_still_draws() {
    assert!(
        widget_is_painted(widget_pdf("<< /N 6 0 R >>", "")),
        "a direct /N stream must still render"
    );
}

/// §12.5.3 Table 165: Hidden means do not display the annotation.
#[test]
fn hidden_annotations_are_not_drawn() {
    assert!(
        !widget_is_painted(widget_pdf("<< /N 6 0 R >>", "/F 2")),
        "a Hidden annotation was painted"
    );
}

/// NoView means do not display it on screen.
#[test]
fn noview_annotations_are_not_drawn() {
    assert!(
        !widget_is_painted(widget_pdf("<< /N 6 0 R >>", "/F 32")),
        "a NoView annotation was painted"
    );
}

/// A Print-flagged, non-hidden annotation is still drawn — the flag check
/// must not reject everything with an `/F`.
#[test]
fn printable_annotations_are_still_drawn() {
    assert!(widget_is_painted(widget_pdf("<< /N 6 0 R >>", "/F 4")));
}

/// One widget whose appearance cannot be decoded costs that widget, not the page.
#[test]
fn test_undecodable_appearance_stream_does_not_fail_the_page() {
    let pdf = widget_pdf_with_off_dict(
        "<< /N << /Off 7 0 R /Yes 6 0 R >> >>",
        "/AS /Off",
        "/Filter /FlateDecode",
    );
    assert!(!widget_is_painted(pdf), "an unreadable appearance paints nothing");
}
