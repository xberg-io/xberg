use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use kreuzberg::types::{Chunk, ChunkMetadata, ExtractionResult, Metadata, PageStructure, PageUnitType};
use kreuzberg_ffi::{CExtractionResultView, kreuzberg_get_result_view};
use std::ffi::CString;
use std::mem;

/// Create a test extraction result with configurable complexity
fn create_test_result(content_size: usize, chunk_count: usize) -> ExtractionResult {
    let mut metadata = Metadata::default();
    metadata.title = Some("Benchmark Test Document".to_string());
    metadata.language = Some("en".to_string());
    metadata.created_at = Some("2025-01-01T00:00:00Z".to_string());
    metadata.subject = Some("Performance Testing".to_string());

    let page_structure = PageStructure {
        total_count: 100,
        unit_type: PageUnitType::Page,
        boundaries: None,
        pages: None,
    };
    metadata.pages = Some(page_structure);

    let content = format!(
        "{}{}{}",
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ",
        "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ".repeat(content_size / 120),
        "Final sentence."
    );

    let chunks = if chunk_count > 0 {
        let chunk_size = content.len() / chunk_count;
        Some(
            (0..chunk_count)
                .map(|i| {
                    let start = i * chunk_size;
                    let end = if i == chunk_count - 1 {
                        content.len()
                    } else {
                        (i + 1) * chunk_size
                    };
                    Chunk {
                        content: content[start..end].to_string(),
                        embedding: None,
                        metadata: ChunkMetadata {
                            byte_start: start,
                            byte_end: end,
                            token_count: Some((end - start) / 4),
                            chunk_index: i,
                            total_chunks: chunk_count,
                            first_page: Some(1 + (i / 10)),
                            last_page: Some(1 + (i / 10)),
                        },
                    }
                })
                .collect(),
        )
    } else {
        None
    };

    ExtractionResult {
        content,
        mime_type: "application/pdf".to_string(),
        metadata,
        tables: vec![],
        detected_languages: Some(vec!["en".to_string(), "de".to_string()]),
        chunks,
        images: None,
        pages: None,
    }
}

/// Benchmark: Zero-copy result view creation
fn bench_zero_copy_view_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("result_view_creation");

    for size in [1_000, 10_000, 100_000, 1_000_000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("zero_copy", size), size, |b, &size| {
            let result = create_test_result(size, 10);
            let result_ptr = &result as *const ExtractionResult;

            b.iter(|| {
                let mut view: CExtractionResultView = unsafe { mem::zeroed() };
                unsafe {
                    kreuzberg_get_result_view(black_box(result_ptr), &mut view);
                }
                black_box(view);
            });
        });
    }

    group.finish();
}

/// Benchmark: Copy-based approach (simulated via CString allocation)
fn bench_copy_based_approach(c: &mut Criterion) {
    let mut group = c.benchmark_group("result_copy_based");

    for size in [1_000, 10_000, 100_000, 1_000_000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("copy_with_cstring", size), size, |b, &size| {
            let result = create_test_result(size, 10);

            b.iter(|| {
                let content_cstr = CString::new(result.content.as_str()).unwrap();
                let mime_cstr = CString::new(result.mime_type.as_str()).unwrap();
                let language_cstr = result
                    .metadata
                    .language
                    .as_ref()
                    .map(|s| CString::new(s.as_str()).unwrap());
                let title_cstr = result
                    .metadata
                    .title
                    .as_ref()
                    .map(|s| CString::new(s.as_str()).unwrap());

                black_box(content_cstr);
                black_box(mime_cstr);
                black_box(language_cstr);
                black_box(title_cstr);
            });
        });
    }

    group.finish();
}

/// Benchmark: Field access from zero-copy view
fn bench_zero_copy_field_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("result_view_field_access");

    let result = create_test_result(100_000, 50);
    let result_ptr = &result as *const ExtractionResult;
    let mut view: CExtractionResultView = unsafe { mem::zeroed() };
    unsafe {
        kreuzberg_get_result_view(result_ptr, &mut view);
    }

    group.bench_function("access_content_length", |b| {
        b.iter(|| {
            black_box(view.content_len);
        });
    });

    group.bench_function("access_all_counts", |b| {
        b.iter(|| {
            black_box(view.table_count);
            black_box(view.chunk_count);
            black_box(view.detected_language_count);
            black_box(view.image_count);
            black_box(view.page_count);
        });
    });

    group.bench_function("read_content_slice", |b| {
        b.iter(|| {
            let content_slice = unsafe { std::slice::from_raw_parts(view.content_ptr, view.content_len) };
            black_box(content_slice);
        });
    });

    group.finish();
}

/// Benchmark: Multiple views from same result (zero overhead)
fn bench_multiple_views(c: &mut Criterion) {
    let result = create_test_result(50_000, 20);
    let result_ptr = &result as *const ExtractionResult;

    c.bench_function("create_multiple_views", |b| {
        b.iter(|| {
            for _ in 0..10 {
                let mut view: CExtractionResultView = unsafe { mem::zeroed() };
                unsafe {
                    kreuzberg_get_result_view(black_box(result_ptr), &mut view);
                }
                black_box(view);
            }
        });
    });
}

/// Benchmark: Zero-copy vs JSON serialization overhead
fn bench_vs_json_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("result_vs_json");

    for size in [10_000, 100_000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        let result = create_test_result(*size, 10);
        let result_ptr = &result as *const ExtractionResult;

        group.bench_with_input(BenchmarkId::new("zero_copy", size), size, |b, _| {
            b.iter(|| {
                let mut view: CExtractionResultView = unsafe { mem::zeroed() };
                unsafe {
                    kreuzberg_get_result_view(black_box(result_ptr), &mut view);
                }
                black_box(view);
            });
        });

        group.bench_with_input(BenchmarkId::new("json_serialize", size), size, |b, _| {
            b.iter(|| {
                let json = serde_json::to_string(&result.metadata).unwrap();
                black_box(json);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_zero_copy_view_creation,
    bench_copy_based_approach,
    bench_zero_copy_field_access,
    bench_multiple_views,
    bench_vs_json_serialization
);
criterion_main!(benches);
