//! Tracks per-glyph text extraction and page object loading in the native PDF engine. ~keep

use std::hint::black_box;
use std::path::Path;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use xberg_native_pdf::document::PdfDocument;

const FIXTURES: [(&str, &str); 2] = [
    ("type0_cjk", "tests/fixtures/vertical_cjk/kampo.pdf"),
    (
        "latin_simple",
        "../../fixtures/repros/caltrans-native-table/fixtures/caltrans-p3.pdf",
    ),
];

fn load_bytes(relative: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read(&path).unwrap_or_else(|error| panic!("read bench fixture {}: {error}", path.display()))
}

fn bench_extract_text(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("extract_text");
    for (name, relative) in FIXTURES {
        let bytes = load_bytes(relative);
        group.bench_function(name, |bencher| {
            bencher.iter_batched(
                || PdfDocument::from_bytes(bytes.clone()).expect("open bench fixture"),
                |document| {
                    let pages = document.page_count().expect("page count");
                    let mut total = 0;
                    for page in 0..pages {
                        total += document.extract_text(page).expect("extract text").len();
                    }
                    assert!(total > 0, "bench fixture produced no text");
                    black_box(total)
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_load_pages(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("load_pages");
    for (name, relative) in FIXTURES {
        let bytes = load_bytes(relative);
        group.bench_function(name, |bencher| {
            bencher.iter_batched(
                || PdfDocument::from_bytes(bytes.clone()).expect("open bench fixture"),
                |document| {
                    for page in document.page_indices() {
                        black_box(document.get_page(page).expect("load page"));
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, bench_extract_text, bench_load_pages);
criterion_main!(benches);
