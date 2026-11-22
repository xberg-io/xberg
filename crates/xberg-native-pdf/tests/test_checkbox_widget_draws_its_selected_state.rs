//! A checkbox draws the appearance `/AS` selects, and a Hidden annotation
//! draws nothing.
//!
//! ISO 32000-1:2008 §12.5.5: when an appearance dictionary's `/N` value is a
//! **sub-dictionary** of appearance states rather than a stream, `/AS` names
//! the one to use. Accepting `/N` only as a stream silently skips every
//! checkbox and radio button in existence, because that is exactly how they
//! are written.
//!
//! §12.5.3 Table 165: Hidden means do not display the annotation at all, and
//! NoView means do not display it on screen. Both have to suppress the paint.
//!
//! The two cases belong in one fixture: a renderer that draws nothing at all
//! passes the Hidden assertion for the wrong reason, so the checkbox must be
//! asserted visible on the same page.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    [
        format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes(),
        data.to_vec(),
        b"\nendstream".to_vec(),
    ]
    .concat()
}

/// One page with two widgets:
///   * a checkbox whose `/AP /N` is a state sub-dictionary and whose `/AS` is
///     `/Yes`, drawing a blue square on the left;
///   * a Hidden (`/F 2`) annotation whose appearance would draw a red square
///     on the right.
fn checkbox_page() -> Vec<u8> {
    let content: &[u8] = b"";
    let off_ap: &[u8] = b"0 1 0 rg 0 0 40 40 re f\n"; // green: must never show
    let yes_ap: &[u8] = b"0 0 1 rg 0 0 40 40 re f\n"; // blue: the selected state
    let hidden_ap: &[u8] = b"1 0 0 rg 0 0 40 40 re f\n"; // red: Hidden, must not show

    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [6 0 R 9 0 R] >> >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
           /Resources << >> /Annots [6 0 R 9 0 R] >>"
            .to_vec(),
        stream("", content),
        b"<< >>".to_vec(), // 5: unused, keeps numbering readable
        // 6: the checkbox widget, /AS selecting /Yes
        b"<< /Type /Annot /Subtype /Widget /FT /Btn /T (cb) /Rect [20 30 60 70] \
           /AS /Yes /AP << /N << /Off 7 0 R /Yes 8 0 R >> >> >>"
            .to_vec(),
        stream("/Type /XObject /Subtype /Form /BBox [0 0 40 40]", off_ap),
        stream("/Type /XObject /Subtype /Form /BBox [0 0 40 40]", yes_ap),
        // 9: a Hidden annotation whose appearance would paint red
        b"<< /Type /Annot /Subtype /Widget /FT /Btn /T (hid) /Rect [140 30 180 70] \
           /F 2 /AP << /N 10 0 R >> >>"
            .to_vec(),
        stream("/Type /XObject /Subtype /Form /BBox [0 0 40 40]", hidden_ap),
    ];

    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    let n = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {n}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

/// Count pixels of each primary on the rendered page.
fn counts() -> (usize, usize, usize) {
    let doc = PdfDocument::from_bytes(checkbox_page()).expect("fixture parses");
    let img = render_page(&doc, 0, &RenderOptions::with_dpi(72)).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let mut blue = 0;
    let mut green = 0;
    let mut red = 0;
    for p in px.pixels() {
        if p[2] > 150 && p[0] < 100 && p[1] < 100 {
            blue += 1;
        }
        if p[1] > 150 && p[0] < 100 && p[2] < 100 {
            green += 1;
        }
        if p[0] > 150 && p[1] < 100 && p[2] < 100 {
            red += 1;
        }
    }
    (blue, green, red)
}

/// `/AS /Yes` selects the blue stream out of the state sub-dictionary.
#[test]
fn test_state_named_by_as_is_drawn() {
    let (blue, green, _) = counts();
    assert!(
        blue > 500,
        "the /Yes appearance was not drawn ({blue} blue pixels). §12.5.5: when \
         /AP /N is a sub-dictionary of states, /AS names the one to use — \
         accepting /N only as a stream skips every checkbox and radio button."
    );
    assert_eq!(
        green, 0,
        "the /Off appearance was drawn ({green} green pixels); /AS names /Yes"
    );
}

/// A Hidden annotation is not painted. Asserted on the same page as a visible
/// one, so a renderer that draws nothing cannot pass this by accident.
#[test]
fn test_hidden_annotation_is_not_painted() {
    let (blue, _, red) = counts();
    assert!(
        blue > 500,
        "the fixture drew nothing at all, so the Hidden assertion below would \
         pass for the wrong reason"
    );
    assert_eq!(
        red, 0,
        "a Hidden annotation (/F 2) was painted ({red} red pixels); §12.5.3 \
         Table 165 says Hidden means do not display it"
    );
}
