//! A region's own `q` must not swallow the save the prescan wrapped it in.
//!
//! Above 256 KB the text extractor stops parsing sequentially and replays only
//! the `BT..ET` regions, injecting the graphics state each one starts with.
//! Each region is bracketed in SaveState/RestoreState precisely so that one
//! region's state cannot leak into the next.
//!
//! A content stream may open a `q` that it closes only at the very end of the
//! page — CAD exporters routinely do, wrapping the whole drawing in one scope
//! after a global `cm`. When such a `q` falls inside a replayed region it is
//! forwarded verbatim, so the region's own RestoreState pops *that* save
//! rather than the injected one, and the injected CTM stays on the stack. The
//! next region's absolute CTM then concatenates onto it, and every region
//! after the first is scaled by the leaked matrix — here 0.12 becomes
//! 0.12 x 0.12 = 0.0144, so 52 pt type is reported at 0.75 pt.
//!
//! ISO 32000-1:2008 §8.4.2: `Q` restores the state saved by the matching `q`.
//! A `Q` whose `q` was never replayed has no match inside the region and must
//! not consume the caller's.
//!
//! Hand-built synthetic PDF; no third-party fixture.

use xberg_native_pdf::document::PdfDocument;

/// Comfortably over the 256 KB prescan threshold.
const FILLER_TARGET: usize = 300 * 1024;

fn build_pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_pos = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_pos
        )
        .as_bytes(),
    );
    out
}

fn stream_obj(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut v = format!("<< {} /Length {} >>\nstream\n", dict, data.len()).into_bytes();
    v.extend_from_slice(data);
    v.extend_from_slice(b"\nendstream");
    v
}

/// A page whose content applies a global 0.12 scale, opens one `q` that is
/// closed only at the very end, and then draws two text runs far apart, with
/// enough filler path data between them to force the prescan route.
fn scaled_page_with_one_outer_save() -> Vec<u8> {
    let mut c = String::new();
    c.push_str("0.12 0 0 0.12 0 0 cm\nq\n");
    c.push_str("BT /F1 100 Tf 1000 8000 Td (FIRST) Tj ET\n");
    // Filler between the two text regions so they cannot merge into one.
    while c.len() < FILLER_TARGET {
        c.push_str("1000 1000 m 2000 2000 l S\n");
    }
    c.push_str("/Span << /MCID 0 >> BDC\nQ\nBT /F1 100 Tf 1000 4000 Td (SECOND) Tj ET\nEMC\n");
    build_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1224 1224] /Contents 4 0 R \
           /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        stream_obj("", c.as_bytes()),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ])
}

/// Both runs are set at 100 pt under a single 0.12 scale, so both must report
/// an effective size of 12. A leaked CTM makes the SECOND run 0.12x smaller.
#[test]
fn test_regions_own_save_does_not_swallow_the_injected_one() {
    let pdf = scaled_page_with_one_outer_save();
    assert!(pdf.len() > 256 * 1024, "fixture must exceed the prescan threshold");
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let spans = doc.extract_spans(0).expect("page extracts");

    let find = |needle: &str| {
        spans.iter().find(|s| s.text.contains(needle)).unwrap_or_else(|| {
            panic!(
                "{needle:?} missing; got {:?}",
                spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>()
            )
        })
    };
    let first = find("FIRST");
    let second = find("SECOND");

    assert!(
        (first.font_size - 12.0).abs() < 0.5,
        "the first run is set at 100pt under a 0.12 scale, so 12pt; got {}",
        first.font_size
    );
    assert!(
        (second.font_size - 12.0).abs() < 0.5,
        "the second run must be 12pt too. Getting ~{:.3} means the region's \
         own `q` swallowed the injected save and the 0.12 CTM was applied \
         twice (0.12 x 0.12 = 0.0144, so 100pt -> 1.44pt); got {}",
        second.font_size,
        second.font_size
    );
    assert!(
        (first.font_size - second.font_size).abs() < 0.01,
        "both runs share one scale; got {} and {}",
        first.font_size,
        second.font_size
    );
}
