//! Issue #291 / GH#1364: a structurally invalid `/Font` resource makes `xberg_native_pdf`
//! drop every glyph painted with that font while the page still renders `Ok`. Asserts
//! the dropped glyphs surface as one deduped `ProcessingWarning` per distinct cause,
//! with a resolvable-font case as the negative control.

#![cfg(feature = "pdf")]

use xberg::ProcessingWarning;
use xberg::pdf::render::{
    install_pdf_render_diagnostics, render_pdf_page_to_png, take_xberg_native_pdf_render_warnings,
};

/// Build a minimal one-page PDF whose `/Resources /Font /F1` entry points at
/// object 5, which is a PDF string (`(NotAFontDict)`) rather than a font
/// dictionary. `xberg_native_pdf`'s `PageRenderer::load_resources`
/// (`page_renderer.rs`) resolves every `/Font` resource through
/// `PdfDocument::get_or_load_font_for_rendering`, which requires the
/// resolved object to be a dictionary; here it fails with `Error::ParseError
/// { reason: "Font object is not a dictionary" }`.
///
/// Per issue #1364, that failure does not abort the page: `load_resources` logs it via
/// `tracing::warn!("rendering text with fallback font data")` -- a sanitized static
/// message, with the actual diagnosis carried in structured tracing fields rather than
/// the message -- and continues, so every glyph the content stream later paints with
/// `/F1` is dropped (the renderer has no font to shape it with) while the page still
/// renders `Ok`. Unlike a
/// missing font *name* (which `xberg_native_pdf` best-effort substitutes via
/// `fontdb` system-font matching and may or may not warn about depending on
/// which fonts happen to be installed), a structurally invalid font
/// **resource** always fails to parse regardless of the host's installed
/// fonts, making this the deterministic repro for CI.
fn build_pdf_with_malformed_font_resource() -> Vec<u8> {
    let content_stream = b"BT /F1 24 Tf 72 700 Td (Hello) Tj 0 -40 Td (World) Tj ET";

    let mut pdf: Vec<u8> = Vec::new();
    macro_rules! push_bytes {
        ($s:expr) => {
            pdf.extend_from_slice($s)
        };
    }
    macro_rules! push_str {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }

    push_bytes!(b"%PDF-1.5\n");

    let off1 = pdf.len();
    push_bytes!(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    push_bytes!(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off4 = pdf.len();
    push_str!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content_stream.len()));
    push_bytes!(content_stream);
    push_bytes!(b"\nendstream\nendobj\n");

    let off3 = pdf.len();
    push_bytes!(
        b"3 0 obj\n\
         << /Type /Page /Parent 2 0 R /MediaBox [0 0 300 792]\n\
            /Resources << /Font << /F1 5 0 R >> >>\n\
            /Contents 4 0 R >>\n\
         endobj\n"
    );

    let off5 = pdf.len();
    push_bytes!(b"5 0 obj\n(NotAFontDict)\nendobj\n");

    let xref_off = pdf.len();
    push_str!(format!(
        "xref\n0 6\n\
         0000000000 65535 f \r\n\
         {off1:010} 00000 n \r\n\
         {off2:010} 00000 n \r\n\
         {off3:010} 00000 n \r\n\
         {off4:010} 00000 n \r\n\
         {off5:010} 00000 n \r\n"
    ));
    push_str!(format!(
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n"
    ));

    pdf
}

/// Build a minimal one-page PDF whose content stream uses a properly declared
/// base-14 font (`/Helvetica`), used as a negative control: rendering a page
/// with a resolvable font must not produce a glyph-drop warning.
fn build_pdf_with_resolvable_font() -> Vec<u8> {
    let content_stream = b"BT /F1 24 Tf 72 700 Td (Hello) Tj ET";

    let mut pdf: Vec<u8> = Vec::new();
    macro_rules! push_bytes {
        ($s:expr) => {
            pdf.extend_from_slice($s)
        };
    }
    macro_rules! push_str {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }

    push_bytes!(b"%PDF-1.5\n");

    let off1 = pdf.len();
    push_bytes!(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    push_bytes!(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off5 = pdf.len();
    push_str!(format!("5 0 obj\n<< /Length {} >>\nstream\n", content_stream.len()));
    push_bytes!(content_stream);
    push_bytes!(b"\nendstream\nendobj\n");

    let off3 = pdf.len();
    push_bytes!(
        b"3 0 obj\n\
         << /Type /Page /Parent 2 0 R /MediaBox [0 0 300 792]\n\
            /Resources << /Font << /F1 4 0 R >> >>\n\
            /Contents 5 0 R >>\n\
         endobj\n"
    );

    let off4 = pdf.len();
    push_bytes!(
        b"4 0 obj\n\
         << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\n\
         endobj\n"
    );

    let xref_off = pdf.len();
    push_str!(format!(
        "xref\n0 6\n\
         0000000000 65535 f \r\n\
         {off1:010} 00000 n \r\n\
         {off2:010} 00000 n \r\n\
         {off3:010} 00000 n \r\n\
         {off4:010} 00000 n \r\n\
         {off5:010} 00000 n \r\n"
    ));
    push_str!(format!(
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n"
    ));

    pdf
}

/// Issue #291 / GH#1364: a page whose `/Font` resource is structurally
/// invalid renders successfully (as it must — a broken font resource must
/// not fail the whole page) but drops every glyph drawn with it. Before this
/// fix nothing surfaced that: `RenderedImage` has no field for dropped glyphs, and
/// xberg_native_pdf's own warning had no subscriber installed to receive it, so the
/// drop was invisible twice over. (It reported through `log::warn!` then and
/// through `tracing::warn!` since the 1.0.1 fork migration — the second time
/// this went dark, the target was unchanged and only the transport moved.)
///
/// This asserts the xberg-side capture end to end: rendering the *same* page
/// twice (e.g. once for a preview and once for OCR, or a retry) hits the
/// identical xberg_native_pdf cause both times, so the two `tracing::warn!` records
/// must dedup into exactly one `ProcessingWarning`, sourced `"pdf-render"`
/// and naming the page — not one warning per render call.
#[test]
fn test_dropped_glyphs_from_malformed_font_produce_one_deduped_warning() {
    // Capture is opt-in — a library must not seize the process-global `tracing` dispatcher
    // on its own. An application (here, the test) asks for it.
    //
    // ★ This assertion holding here is NOT evidence that capture works in the CLI. A test
    // binary installs no subscriber of its own, so `install_pdf_render_diagnostics` always
    // wins the single global dispatcher slot; `xberg-cli` claims that slot first and composes
    // `glyph_drop_capture_layer()` into its own stack instead. Both paths need exercising.
    assert!(
        install_pdf_render_diagnostics(),
        "no other component should own the tracing dispatcher in this test binary"
    );

    // Drain any residual state from a previous render on this thread so the
    // assertion below is exact, not "at least".
    let _ = take_xberg_native_pdf_render_warnings();

    let pdf = build_pdf_with_malformed_font_resource();

    let first_png = render_pdf_page_to_png(&pdf, 0, Some(150), None)
        .expect("page with a malformed font resource must still render");
    assert!(!first_png.is_empty(), "renderer must still produce page bytes");

    let second_png =
        render_pdf_page_to_png(&pdf, 0, Some(150), None).expect("re-rendering the same page must also succeed");
    assert!(!second_png.is_empty(), "renderer must still produce page bytes");

    let warnings = take_xberg_native_pdf_render_warnings();
    assert_eq!(
        warnings.len(),
        1,
        "two renders hitting the identical malformed-font cause must dedup to one warning, got: {warnings:?}"
    );

    let warning = &warnings[0];
    assert_eq!(
        warning.source, "pdf-render",
        "glyph-drop warnings must be sourced \"pdf-render\", got: {}",
        warning.source
    );
    assert!(
        warning.message.contains("Page 1"),
        "warning must name the affected page, got: {}",
        warning.message
    );
    assert!(
        warning.message.contains("glyph"),
        "warning must describe glyph loss, not just repeat the raw xberg_native_pdf log line, got: {}",
        warning.message
    );
    assert!(
        warning.message.contains("rendering text with fallback font data"),
        "warning must carry xberg_native_pdf's sanitized static message; the actual \
         diagnosis now lives in structured tracing fields, not the message, got: {}",
        warning.message
    );

    // A second drain on the same thread must come back empty: warnings are
    // consumed, not accumulated forever across unrelated render calls.
    let drained_again = take_xberg_native_pdf_render_warnings();
    assert!(
        drained_again.is_empty(),
        "take_xberg_native_pdf_render_warnings must drain, not peek: got {drained_again:?}"
    );
}

/// Negative control: a page whose font resolves cleanly (a declared base-14
/// font) must not produce a glyph-drop warning. Without this, a change that
/// makes the capture logger too broad (e.g. matching on level alone) would
/// pass the positive test above while flooding every normal render with
/// spurious warnings.
#[test]
fn test_resolvable_font_page_produces_no_glyph_drop_warning() {
    // Install first, otherwise "no warnings" would pass trivially because capture was never
    // armed — a test that passes when the code is broken.
    //
    // ★ That guard is necessary but NOT sufficient, and this test is the standing example:
    // when the capture went dark on the xberg_native_pdf 1.0.1 log->tracing migration it kept
    // passing, because an empty buffer satisfies `warnings.is_empty()` just as well as a
    // working sink that found nothing. Only the positive tests caught the regression.
    assert!(
        install_pdf_render_diagnostics(),
        "no other component should own the tracing dispatcher in this test binary"
    );
    let _ = take_xberg_native_pdf_render_warnings();

    let pdf = build_pdf_with_resolvable_font();
    let png = render_pdf_page_to_png(&pdf, 0, Some(150), None).expect("page with a resolvable font must render");
    assert!(!png.is_empty(), "renderer must still produce page bytes");

    let warnings: Vec<ProcessingWarning> = take_xberg_native_pdf_render_warnings();
    assert!(
        warnings.is_empty(),
        "a page whose font resolves cleanly must not produce a glyph-drop warning, got: {warnings:?}"
    );
}
