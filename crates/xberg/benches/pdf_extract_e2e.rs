use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use xberg::{ExtractInput, ExtractionConfig, OutputFormat, PdfBackend, PdfConfig, extract};

/// The native-pdf-fast-b8 cohort documents, relative to `test_documents/`.
const DOCUMENTS: &[(&str, &str)] = &[
    ("fake_memo", "pdf/fake_memo.pdf"),
    ("tiny", "pdf/tiny.pdf"),
    ("embedded_images_tables", "pdf/embedded_images_tables.pdf"),
    ("google_doc_document", "pdf/google_doc_document.pdf"),
    ("681693", "pdf/681693.pdf"),
    ("ft_ACN_2009_page_102_t0", "pdf/ft_ACN_2009_page_102_t0.pdf"),
    (
        "pb_blackrock_page4",
        ".corpus-cache/pdf/pb_fqr-retail-blackrock-global-allocation-fund-inc_page4.pdf",
    ),
    ("pb_sample_page_16", ".corpus-cache/pdf/pb_sample_page_16_page1.pdf"),
];

const BACKENDS: &[(&str, PdfBackend)] = &[("native", PdfBackend::Native), ("pdfium", PdfBackend::Pdfium)];

fn document_path(relative: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_documents")
        .join(relative);
    assert!(path.is_file(), "benchmark PDF is missing: {}", path.display());
    path
}

fn config(backend: PdfBackend, output_format: OutputFormat) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format,
        pdf_options: Some(PdfConfig {
            backend,
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn run(path: &Path, config: &ExtractionConfig) -> xberg::ExtractionResult {
    extract(ExtractInput::from_uri(path.to_string_lossy().into_owned()), config)
        .await
        .unwrap_or_else(|error| panic!("extraction of {} failed: {error}", path.display()))
}

fn bench_group(criterion: &mut Criterion, group_name: &str, output_format: OutputFormat) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("benchmark runtime");
    let mut group = criterion.benchmark_group(group_name);
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));

    for (document, relative) in DOCUMENTS {
        let path = document_path(relative);
        for (backend_name, backend) in BACKENDS {
            let extraction_config = config(*backend, output_format.clone());
            let warmup = runtime.block_on(run(&path, &extraction_config));
            assert!(
                warmup
                    .results
                    .iter()
                    .any(|document| !document.content.trim().is_empty()),
                "{backend_name} backend returned empty content for {}",
                path.display()
            );
            group.bench_function(BenchmarkId::new(*backend_name, document), |bencher| {
                bencher.iter(|| black_box(runtime.block_on(run(&path, &extraction_config))));
            });
        }
    }
    group.finish();
}

fn bench_plain(criterion: &mut Criterion) {
    bench_group(criterion, "pdf_extract_plain", OutputFormat::Plain);
}

fn bench_markdown(criterion: &mut Criterion) {
    bench_group(criterion, "pdf_extract_markdown", OutputFormat::Markdown);
}

criterion_group!(benches, bench_plain, bench_markdown);
criterion_main!(benches);
