#![cfg(all(feature = "pdf", feature = "tokio-runtime", not(target_arch = "wasm32")))]

use std::io::{Read, Seek, Write};
use xberg::core::config::{ExtractInput, ExtractionConfig, PageConfig};
use xberg::extractors::security::SecurityLimits;

fn write_pdf(path: &std::path::Path, unused_stream_size: usize) {
    let mut pdf = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    pdf.write_all(b"%PDF-1.7\n").unwrap();
    let content = b"BT /F1 24 Tf 72 720 Td (Path source) Tj ET";
    let stream = format!(
        "<< /Length {} >>\nstream\n{}\nendstream",
        content.len(),
        String::from_utf8_lossy(content)
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        &stream,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ];
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.stream_position().unwrap());
        writeln!(pdf, "{} 0 obj\n{object}\nendobj", index + 1).unwrap();
    }
    offsets.push(pdf.stream_position().unwrap());
    writeln!(pdf, "6 0 obj\n<< /Length {unused_stream_size} >>\nstream").unwrap();
    std::io::copy(&mut std::io::repeat(b'X').take(unused_stream_size as u64), &mut pdf).unwrap();
    writeln!(pdf, "\nendstream\nendobj").unwrap();
    let xref = pdf.stream_position().unwrap();
    writeln!(pdf, "xref\n0 7\n0000000000 65535 f ").unwrap();
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").unwrap();
    }
    writeln!(pdf, "trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF").unwrap();
    pdf.flush().unwrap();
}

#[tokio::test]
async fn small_and_large_pdf_paths_preserve_native_text_and_page_limit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.pdf");
    let mut config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        security_limits: Some(SecurityLimits {
            max_pages: Some(1),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = || ExtractInput::from_uri(path.to_str().unwrap());
    for unused_stream_size in [0, 2 * 1024 * 1024] {
        write_pdf(&path, unused_stream_size);
        let result = Box::pin(xberg::extract(input(), &config)).await.unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let [document] = result.results.as_slice() else {
            panic!("expected one document");
        };
        assert_eq!(document.content.trim(), "Path source");
        let [page] = document.pages.as_deref().unwrap() else {
            panic!("expected one page");
        };
        assert_eq!(page.content.trim(), "Path source");
    }
    config.security_limits.as_mut().unwrap().max_pages = Some(0);
    let result = Box::pin(xberg::extract(input(), &config)).await;
    assert!(result.is_err() || result.is_ok_and(|result| result.results.is_empty() && result.errors.len() == 1));
}
