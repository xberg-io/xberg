#![cfg(all(feature = "pdf", feature = "tokio-runtime", not(target_arch = "wasm32")))]

use std::io::{Read, Seek, Write};
use xberg::core::config::{ExtractInput, ExtractionConfig, PageConfig, PdfConfig};
use xberg::extractors::security::{SecurityError, SecurityLimits};

fn write_pdf(path: &std::path::Path, unused_stream_size: usize, outlines: bool) {
    let mut pdf = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    pdf.write_all(b"%PDF-1.7\n").unwrap();
    let content = b"BT /F1 24 Tf 72 720 Td (Path source) Tj ET";
    let stream = format!(
        "<< /Length {} >>\nstream\n{}\nendstream",
        content.len(),
        String::from_utf8_lossy(content)
    );
    let catalog = if outlines {
        "<< /Type /Catalog /Pages 2 0 R /Outlines 7 0 R >>"
    } else {
        "<< /Type /Catalog /Pages 2 0 R >>"
    };
    let objects = [
        catalog,
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
    if outlines {
        offsets.push(pdf.stream_position().unwrap());
        writeln!(
            pdf,
            "7 0 obj\n<< /Type /Outlines /First 8 0 R /Last 8 0 R /Count 1 >>\nendobj"
        )
        .unwrap();
        offsets.push(pdf.stream_position().unwrap());
        writeln!(
            pdf,
            "8 0 obj\n<< /Title (Path bookmark) /Parent 7 0 R /Dest [3 0 R /Fit] >>\nendobj"
        )
        .unwrap();
    }
    let object_count = offsets.len() + 1;
    let xref = pdf.stream_position().unwrap();
    writeln!(pdf, "xref\n0 {object_count}\n0000000000 65535 f ").unwrap();
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").unwrap();
    }
    writeln!(
        pdf,
        "trailer\n<< /Size {object_count} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF"
    )
    .unwrap();
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
    for (unused_stream_size, outlines) in [(0, false), (2 * 1024 * 1024, false), (0, true)] {
        write_pdf(&path, unused_stream_size, outlines);
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
        assert_bytes_parity(&path, &config, document).await;
    }
    config.security_limits.as_mut().unwrap().max_pages = Some(0);
    let error = Box::pin(xberg::extract(input(), &config)).await.unwrap_err();
    assert!(matches!(
        error,
        xberg::XbergError::Security { source: Some(source), .. }
            if matches!(source.downcast_ref::<SecurityError>(), Some(SecurityError::TooManyPages { count: 1, max: 0 }))
    ));
}

async fn assert_bytes_parity(
    path: &std::path::Path,
    config: &ExtractionConfig,
    document: &xberg::types::ExtractedDocument,
) {
    let input = ExtractInput::from_bytes(std::fs::read(path).unwrap(), "application/pdf", None);
    let result = Box::pin(xberg::extract(input, config)).await.unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let [bytes] = result.results.as_slice() else {
        panic!("expected one bytes document");
    };
    assert_eq!(document.content, bytes.content);
    assert_eq!(
        serde_json::to_value(&document.pages).unwrap(),
        serde_json::to_value(&bytes.pages).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&document.processing_warnings).unwrap(),
        serde_json::to_value(&bytes.processing_warnings).unwrap(),
    );
}

#[tokio::test]
async fn encrypted_pdf_path_matches_bytes_with_password() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pdf/encrypted_needs_password.pdf");
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        pdf_options: Some(PdfConfig {
            passwords: Some(vec!["secret".into()]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let input = ExtractInput::from_uri(path.to_str().unwrap());
    let result = Box::pin(xberg::extract(input, &config)).await.unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let [document] = result.results.as_slice() else {
        panic!("expected one encrypted document");
    };
    assert_bytes_parity(&path, &config, document).await;
}

#[tokio::test]
async fn pdf_path_with_embedded_files_keeps_attachment_discovery() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("attachments.pdf");
    write_pdf(&path, 0, false);
    let mut pdf = lopdf::Document::load(&path).unwrap();
    let data = pdf.add_object(lopdf::Stream::new(lopdf::dictionary! {}, b"Attachment text".to_vec()));
    let filespec = pdf.add_object(lopdf::dictionary! { "EF" => lopdf::dictionary! { "F" => data } });
    pdf.catalog_mut().unwrap().set(
        "Names",
        lopdf::dictionary! {
            "EmbeddedFiles" => lopdf::dictionary! {
                "Names" => vec![lopdf::Object::string_literal("attachment.txt"), filespec.into()],
            },
        },
    );
    pdf.save(&path).unwrap();

    let config = ExtractionConfig {
        use_cache: false,
        disable_ocr: true,
        ..Default::default()
    };
    let result = Box::pin(xberg::extract(ExtractInput::from_uri(path.to_str().unwrap()), &config))
        .await
        .unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let [document] = result.results.as_slice() else {
        panic!("expected one PDF");
    };
    assert_eq!(document.content.trim(), "Path source");
    let [child] = document.children.as_deref().unwrap() else {
        panic!("expected one attachment");
    };
    assert_eq!(child.path, "attachment.txt");
    assert_eq!(child.result.content.trim(), "Attachment text");
}
