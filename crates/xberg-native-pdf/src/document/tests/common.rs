use super::super::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tracing::Level;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt as _;

#[derive(Clone, Debug)]
pub(super) struct CapturedEvent {
    pub(super) level: Level,
    pub(super) target: String,
    pub(super) fields: BTreeMap<String, String>,
}

#[derive(Clone, Default)]
struct EventCapture(Arc<Mutex<Vec<CapturedEvent>>>);

impl<S> Layer<S> for EventCapture
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _context: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = FieldCapture::default();
        event.record(&mut visitor);
        self.0.lock().unwrap().push(CapturedEvent {
            level: *event.metadata().level(),
            target: event.metadata().target().to_string(),
            fields: visitor.0,
        });
    }
}

#[derive(Default)]
struct FieldCapture(BTreeMap<String, String>);

impl tracing::field::Visit for FieldCapture {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().to_string(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.insert(field.name().to_string(), value.to_string());
    }
}

pub(super) fn capture_events<T>(operation: impl FnOnce() -> T) -> (T, Vec<CapturedEvent>) {
    let capture = EventCapture::default();
    let subscriber = tracing_subscriber::registry().with(capture.clone());
    let result = tracing::subscriber::with_default(subscriber, operation);
    let events = capture.0.lock().unwrap().clone();
    (result, events)
}

/// Build a minimal PDF with circular Form XObjects: X0 references X1, X1 references X0.
pub(super) fn build_circular_xobject_pdf() -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /XObject << /X0 5 0 R /X1 6 0 R >> >> >>\nendobj\n");

    let off4 = pdf.len();
    let content = b"/X0 Do";
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let off5 = pdf.len();
    let x0_content = b"/X1 Do";
    pdf.extend_from_slice(format!("5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << /XObject << /X1 6 0 R >> >> /Length {} >>\nstream\n", x0_content.len()).as_bytes());
    pdf.extend_from_slice(x0_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let off6 = pdf.len();
    let x1_content = b"/X0 Do";
    pdf.extend_from_slice(format!("6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << /XObject << /X0 5 0 R >> >> /Length {} >>\nstream\n", x1_content.len()).as_bytes());
    pdf.extend_from_slice(x1_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off5).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off6).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    pdf
}

/// Build a minimal one-page PDF whose Form XObject is invoked as
/// `q <6 numbers> /Name Do Q` — deliberately missing the `cm` operator
/// token, so the numbers are dangling operands with nothing to consume
/// them. Per ISO 32000-1:2008 §7.8.2 an operator's operand is whatever
/// immediately precedes it in the stream; `Do`'s operand here is still
/// the Name, not the stray numbers ahead of it.
///
/// `direct_text`: when `Some`, the page's own content stream also draws
/// this text directly before invoking the XObject; when `None`, the page
/// draws nothing itself and all text comes from the XObject.
pub(super) fn build_xobject_do_with_orphaned_operands_pdf(direct_text: Option<&str>) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
              /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> \
              /XObject << /Overlay 6 0 R >> >> >>\nendobj\n",
    );

    let off4 = pdf.len();
    let mut content = Vec::new();
    if let Some(text) = direct_text {
        content.extend_from_slice(format!("BT /F1 12 Tf 1 0 0 1 20 250 Tm ({text}) Tj ET\n").as_bytes());
    }
    // Deliberately missing `cm`: dangling "1 0 0 1 20 150" operands
    // directly precede "/Overlay Do". ~keep
    content.extend_from_slice(b"q 1 0 0 1 20 150 /Overlay Do Q");

    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(&content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let off5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
              /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    let off6 = pdf.len();
    let xobj_content = b"BT /F1 12 Tf 1 0 0 1 10 12 Tm (overlay text) Tj ET";
    pdf.extend_from_slice(
        format!(
            "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 300 40] /Length {} >>\nstream\n",
            xobj_content.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(xobj_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in [off1, off2, off3, off4, off5, off6] {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    pdf
}

/// Build a one-page PDF whose page stream only paints Form XObject "Fm0".
/// The form holds `lines` text lines, all inside one `/PlacedPDF` marked-content
/// scope; the page stream itself has no `/PlacedPDF` tag.
pub(super) fn build_placed_pdf_inside_form_pdf(lines: &[String]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] \
              /Contents 4 0 R /Resources << /XObject << /Fm0 6 0 R >> >> >>\nendobj\n",
    );
    let off4 = pdf.len();
    let content = b"/Figure BDC q /Fm0 Do Q EMC";
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let off5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
              /Encoding /WinAnsiEncoding >>\nendobj\n",
    );
    let off6 = pdf.len();
    let mut form = String::from("/PlacedPDF /MC0 BDC\nBT /F1 10 Tf\n");
    for (i, line) in lines.iter().enumerate() {
        let y = 780 - 12 * i;
        form.push_str(&format!("1 0 0 1 20 {y} Tm ({line}) Tj\n"));
    }
    form.push_str("ET\nEMC\n");
    pdf.extend_from_slice(
        format!(
            "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 600 800] \
             /Resources << /Font << /F1 5 0 R >> >> /Length {} >>\nstream\n",
            form.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(form.as_bytes());
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for off in [off1, off2, off3, off4, off5, off6] {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    pdf
}

/// Build a one-page PDF with the given page content stream and the given
/// content stream for Form XObject "Fm0". Both use Helvetica as `/F1`.
pub(super) fn build_page_and_form_pdf(page_stream: &str, form_stream: &str) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R \
              /Resources << /Font << /F1 5 0 R >> /XObject << /Fm0 6 0 R >> >> >>\nendobj\n",
    );
    let off4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", page_stream.len()).as_bytes());
    pdf.extend_from_slice(page_stream.as_bytes());
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let off5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
              /Encoding /WinAnsiEncoding >>\nendobj\n",
    );
    let off6 = pdf.len();
    pdf.extend_from_slice(
        format!(
            "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 600 800] \
             /Resources << /Font << /F1 5 0 R >> >> /Length {} >>\nstream\n",
            form_stream.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(form_stream.as_bytes());
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for off in [off1, off2, off3, off4, off5, off6] {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    pdf
}

/// Build a two-page PDF whose pages both paint Form XObject "Fm0" with the same
/// form stream. Page 1 paints `page_a`, page 2 paints `page_b`.
pub(super) fn build_two_pages_sharing_form_pdf(page_a: &str, page_b: &str, form_stream: &str) -> Vec<u8> {
    build_two_pages_with_forms_pdf(page_a, page_b, form_stream, None)
}

/// Like [`build_two_pages_sharing_form_pdf`], with an optional second form "Fm1"
/// that both pages and "Fm0" can paint.
pub(super) fn build_two_pages_with_forms_pdf(
    page_a: &str,
    page_b: &str,
    form_stream: &str,
    inner_stream: Option<&str>,
) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offs = Vec::new();
    let (xobjects, form_xobjects) = if inner_stream.is_some() {
        ("/Fm0 6 0 R /Fm1 9 0 R", " /XObject << /Fm1 9 0 R >>")
    } else {
        ("/Fm0 6 0 R", "")
    };
    let res = format!("/Resources << /Font << /F1 5 0 R >> /XObject << {xobjects} >> >>");
    let mut push = |pdf: &mut Vec<u8>, body: String| {
        offs.push(pdf.len());
        pdf.extend_from_slice(body.as_bytes());
    };
    push(&mut pdf, "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".into());
    push(
        &mut pdf,
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R 7 0 R] /Count 2 >>\nendobj\n".into(),
    );
    push(
        &mut pdf,
        format!("3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R {res} >>\nendobj\n"),
    );
    push(
        &mut pdf,
        format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{page_a}\nendstream\nendobj\n",
            page_a.len()
        ),
    );
    push(
        &mut pdf,
        "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n".into(),
    );
    push(
        &mut pdf,
        format!(
            "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 600 800] /Resources << /Font << /F1 5 0 R >>{form_xobjects} >> /Length {} >>\nstream\n{form_stream}\nendstream\nendobj\n",
            form_stream.len()
        ),
    );
    push(
        &mut pdf,
        format!("7 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 8 0 R {res} >>\nendobj\n"),
    );
    push(
        &mut pdf,
        format!(
            "8 0 obj\n<< /Length {} >>\nstream\n{page_b}\nendstream\nendobj\n",
            page_b.len()
        ),
    );
    let mut size = 9;
    if let Some(inner) = inner_stream {
        push(
            &mut pdf,
            format!(
                "9 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 600 800] /Resources << /Font << /F1 5 0 R >> >> /Length {} >>\nstream\n{inner}\nendstream\nendobj\n",
                inner.len()
            ),
        );
        size = 10;
    }
    let xref_off = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for off in offs {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref_off
        )
        .as_bytes(),
    );
    pdf
}

/// Text lines as `Tm`/`Tj` operators at `x`, starting at `top` and stepping down 12 units.
pub(super) fn text_lines_stream(lines: &[String], x: i32, top: i32) -> String {
    let mut out = String::from("BT /F1 10 Tf\n");
    for (i, line) in lines.iter().enumerate() {
        let y = top - 12 * i as i32;
        out.push_str(&format!("1 0 0 1 {x} {y} Tm ({line}) Tj\n"));
    }
    out.push_str("ET\n");
    out
}

/// Build a one-page PDF whose page paints `page_stream`, with one Form XObject per
/// `(name, bbox, stream)` entry. The page and every form can paint every form,
/// and all use Helvetica as `/F1`.
pub(super) fn build_forms_pdf(page_stream: &str, forms: &[(&str, &str, &str)]) -> Vec<u8> {
    let xobjects: String = forms
        .iter()
        .enumerate()
        .map(|(i, (name, _, _))| format!("/{name} {} 0 R ", i + 6))
        .collect();
    let res = format!("/Resources << /Font << /F1 5 0 R >> /XObject << {xobjects}>> >>");
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R {res} >>"),
        format!("<< /Length {} >>\nstream\n{page_stream}\nendstream", page_stream.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
    ];
    for (_, bbox, stream) in forms {
        objects.push(format!(
            "<< /Type /XObject /Subtype /Form /BBox {bbox} {res} /Length {} >>\nstream\n{stream}\nendstream",
            stream.len()
        ));
    }
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offs = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offs.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref_off = pdf.len();
    let size = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for off in offs {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n").as_bytes(),
    );
    pdf
}

/// Build a minimal one-page PDF where XObject "Outer" invokes a second
/// XObject "Inner" (both via the same malformed missing-`cm` `Do` shape
/// as [`build_xobject_do_with_orphaned_operands_pdf`]), and "Inner" is
/// where the actual text lives.
pub(super) fn build_nested_xobject_do_with_orphaned_operands_pdf() -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
              /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> \
              /XObject << /Outer 6 0 R >> >> >>\nendobj\n",
    );

    let off4 = pdf.len();
    let content = b"q 1 0 0 1 0 0 /Outer Do Q";
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let off5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
              /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    let off6 = pdf.len();
    // "Outer" invokes "Inner" — same malformed missing-`cm` shape, one level deeper. ~keep
    let outer_content = b"q 1 0 0 1 10 10 /Inner Do Q";
    pdf.extend_from_slice(
        format!(
            "6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 300 300] \
                  /Resources << /XObject << /Inner 7 0 R >> >> /Length {} >>\nstream\n",
            outer_content.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(outer_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let off7 = pdf.len();
    let inner_content = b"BT /F1 12 Tf 1 0 0 1 10 12 Tm (nested text) Tj ET";
    pdf.extend_from_slice(
        format!(
            "7 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 300 40] /Length {} >>\nstream\n",
            inner_content.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(inner_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 8\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in [off1, off2, off3, off4, off5, off6, off7] {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    pdf
}

/// Build a minimal PDF in memory with given content stream bytes.
/// Returns the raw PDF bytes suitable for `PdfDocument::from_bytes`.
pub(super) fn build_minimal_pdf(content: &[u8]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << >> >>\nendobj\n",
    );

    let off4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 5\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    pdf
}

/// Build a minimal PDF with a `/Font` resource (needed for `Tf`/`Tj`
/// to resolve glyph widths), used by the NaN-bbox regression test.
/// Build a one-page PDF embedding TWO subsets of the SAME base font -
/// `ABCDEF+Helvetica` and `GHIJKL+Helvetica` - whose font programs differ
/// in size. `big_in_f1` chooses which resource slot carries the larger
/// program, so a caller can show the choice does not depend on the order
/// the fonts are encountered.
///
/// Returns `(pdf_bytes, small_program, big_program)`. The programs are not
/// real TrueType: `FontFile2` is decoded and stored verbatim, never parsed,
/// so distinguishable payloads keep the test on the dedup logic.
pub(super) fn build_pdf_with_two_font_subsets(big_in_f1: bool) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let small: Vec<u8> = b"SMALL-SUBSET-".iter().cycle().take(64).copied().collect();
    let big: Vec<u8> = b"BIG-SUBSET-".iter().cycle().take(512).copied().collect();
    let (f1_prog, f2_prog) = if big_in_f1 {
        (big.clone(), small.clone())
    } else {
        (small.clone(), big.clone())
    };

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offs: Vec<usize> = Vec::new();

    offs.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    offs.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    offs.push(pdf.len());
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
              /Resources << /Font << /F1 4 0 R /F2 7 0 R >> >> >>\nendobj\n",
    );

    // /F1 = ABCDEF+Helvetica, /F2 = GHIJKL+Helvetica. Same canonical base
    // name, so they must dedup to a single entry. ~keep
    for (obj, prefix, desc_obj, file_obj, prog) in [(4, "ABCDEF", 5, 6, &f1_prog), (7, "GHIJKL", 8, 9, &f2_prog)] {
        offs.push(pdf.len());
        pdf.extend_from_slice(
            format!(
                "{obj} 0 obj\n<< /Type /Font /Subtype /TrueType /BaseFont /{prefix}+Helvetica \
                     /FontDescriptor {desc_obj} 0 R >>\nendobj\n"
            )
            .as_bytes(),
        );

        offs.push(pdf.len());
        pdf.extend_from_slice(
            format!(
                "{desc_obj} 0 obj\n<< /Type /FontDescriptor /FontName /{prefix}+Helvetica \
                     /Flags 32 /FontFile2 {file_obj} 0 R >>\nendobj\n"
            )
            .as_bytes(),
        );

        offs.push(pdf.len());
        pdf.extend_from_slice(
            format!(
                "{file_obj} 0 obj\n<< /Length {} /Length1 {} >>\nstream\n",
                prog.len(),
                prog.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(prog);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
    }

    // Objects were emitted 1,2,3 then 4,5,6 then 7,8,9 - already in order. ~keep
    let xref_off = pdf.len();
    let total = offs.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {total}\n").as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offs {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {total} /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n").as_bytes(),
    );

    (pdf, small, big)
}

pub(super) fn build_minimal_pdf_with_font(content: &[u8]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    // Deliberately no /MediaBox (and none on /Pages 2 0 R to inherit):
    // `postprocess_spans`'s off-page span filter is skipped entirely
    // when `get_page_media_box` errors, so a page missing /MediaBox
    // is the one path where a NaN bbox component survives to the
    // reading-order sort instead of being silently dropped. ~keep
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R \
              /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
    );

    let off4 = pdf.len();
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    let off5 = pdf.len();
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
              /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in [off1, off2, off3, off4, off5] {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    pdf
}

/// Build a minimal PDF with a multi-page structure (given page count).
pub(super) fn build_multi_page_pdf(page_count: usize) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    offsets.push(pdf.len());
    let kids_str: String = (0..page_count)
        .map(|i| format!("{} 0 R", i + 3))
        .collect::<Vec<_>>()
        .join(" ");
    let pages_obj = format!(
        "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
        kids_str, page_count
    );
    pdf.extend_from_slice(pages_obj.as_bytes());

    for _i in 0..page_count {
        offsets.push(pdf.len());
        let page_obj = format!(
            "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n",
            offsets.len()
        );
        pdf.extend_from_slice(page_obj.as_bytes());
    }

    let xref_off = pdf.len();
    let total_objs = offsets.len() + 1; // +1 for object 0 ~keep
    pdf.extend_from_slice(format!("xref\n0 {}\n", total_objs).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            total_objs, xref_off
        )
        .as_bytes(),
    );

    pdf
}

pub(super) fn make_test_span(text: &str, x: f32, y: f32, width: f32, font_size: f32) -> TextSpan {
    TextSpan {
        provenance: None,
        text_rise: 0.0,
        artifact_type: None,
        text: text.to_string(),
        bbox: crate::geometry::Rect {
            x,
            y,
            width,
            height: font_size,
        },
        font_name: "F1".to_string(),
        font_size,
        font_weight: crate::layout::FontWeight::Normal,
        is_italic: false,
        is_monospace: false,
        color: crate::layout::Color::new(0.0, 0.0, 0.0),
        mcid: None,
        mcid_scope: None,
        sequence: 0,
        split_boundary_before: false,
        offset_semantic: false,
        char_spacing: 0.0,
        word_spacing: 0.0,
        horizontal_scaling: 100.0,
        primary_detected: false,
        char_widths: vec![],
        char_x_offsets: Vec::new(),
        heading_level: None,
        rotation_degrees: 0.0,
        wmode: 0,
        rtl_draw_logical: false,
        mirrored: false,
        page_rotation_applied: 0,
    }
}

// --- topological_block_order (multi-region reading order) -----------------
// Larger Y = higher on the page (read first). Two columns separated by a
// gutter (a > ~1 em horizontal gap) must be read column-major (left column
// fully, then right), and only genuine multi-column PROSE should engage it —
// single-column, fragmented tables and TOC page-number rails must be left on
// the row-aware path (return None) so their output is unchanged. ~keep
pub(super) fn two_dense_columns(left_dense: bool, right_dense: bool) -> Vec<TextSpan> {
    let mut spans = Vec::new();
    for k in 0..8 {
        let y = 200.0 - k as f32 * 12.0;
        let l = if left_dense {
            format!("left column body sentence number {k} here")
        } else {
            format!("{k}")
        };
        let r = if right_dense {
            format!("right column body sentence number {k} here")
        } else {
            format!("{}", (k + 1) * 10) // short page-number-like values ~keep
        };
        spans.push(make_test_span(&l, 0.0, y, 90.0, 10.0));
        spans.push(make_test_span(&r, 120.0, y, 90.0, 10.0));
    }
    spans
}

pub(super) fn make_decimal_span(text: &str, char_widths: Vec<f32>, bbox_w: f32, font_size: f32) -> TextSpan {
    TextSpan {
        provenance: None,
        text_rise: 0.0,
        text: text.to_string(),
        bbox: crate::geometry::Rect {
            x: 0.0,
            y: 0.0,
            width: bbox_w,
            height: font_size,
        },
        font_name: "F1".to_string(),
        font_size,
        font_weight: crate::layout::FontWeight::Normal,
        is_italic: false,
        is_monospace: false,
        color: crate::layout::Color::new(0.0, 0.0, 0.0),
        mcid: None,
        mcid_scope: None,
        sequence: 0,
        split_boundary_before: false,
        offset_semantic: false,
        char_spacing: 0.0,
        word_spacing: 0.0,
        horizontal_scaling: 100.0,
        primary_detected: false,
        artifact_type: None,
        char_widths,
        char_x_offsets: Vec::new(),
        heading_level: None,
        rotation_degrees: 0.0,
        wmode: 0,
        rtl_draw_logical: false,
        mirrored: false,
        page_rotation_applied: 0,
    }
}

// ========================================================================
// reverse_rtl_visual_order_runs tests
// ========================================================================
//
// These tests cover the two distinct RTL span shapes xberg-native-pdf sees
// in the wild and make sure future changes don't regress either:
//
// 1. **Pre-shaped visual-order single span** — one `TextSpan` per
//    line whose `text` already contains contextual Arabic glyphs
//    (U+FB50-U+FDFF / U+FE70-U+FEFF) in the order the content
//    stream drew them (rightmost glyph first). This is the
//    `ArabicCIDTrueType.pdf` pdfjs test fixture case. Expected:
//    character sequence gets reversed in place.
//
// 2. **Plain base-Arabic logical-order single span** — one
//    `TextSpan` per line whose `text` uses base Arabic (U+0621-
//    U+06FF) characters in logical / reading order, as most
//    well-behaved PDF producers emit. Expected: span is left
//    completely alone (no reversal, no shape changes).
//
// The gate that protects case 2 from case 1's reversal is the
// `has_presentation_form` check inside `reverse_rtl_visual_order_runs`. ~keep
pub(super) fn make_rtl_test_span(text: &str, x: f32, y: f32) -> TextSpan {
    TextSpan {
        text: text.to_string(),
        bbox: crate::geometry::Rect::new(x, y, 100.0, 12.0),
        font_size: 12.0,
        ..TextSpan::default()
    }
}

pub(super) fn span_wmode(x: f32, y: f32, wmode: u8) -> TextSpan {
    TextSpan {
        text: "x".to_string(),
        bbox: crate::geometry::Rect::new(x, y, 12.0, 12.0),
        wmode,
        ..TextSpan::default()
    }
}
