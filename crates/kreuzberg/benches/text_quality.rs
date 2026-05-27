use criterion::{Criterion, criterion_group, criterion_main};
use kreuzberg::text::quality::calculate_quality_score;
use std::hint::black_box;

// ~1 KiB of typical paragraph text — no script or style content.
fn corpus_clean_1kib() -> String {
    let paragraph = "The document processing pipeline extracts structured content from a wide \
        variety of file formats including PDF, Office documents, plain text, and HTML. \
        Quality scoring evaluates the extracted text along several dimensions: OCR \
        artifact density, presence of script or style noise, navigational boilerplate, \
        and structural coherence measured by sentence and paragraph metrics. ";
    paragraph.repeat(3)
}

// ~64 KiB — paragraph text interleaved with script/style noise blocks.
fn corpus_noisy_64kib() -> String {
    let para = "This is a representative paragraph of body text extracted from a web page. \
        It contains normal prose with proper punctuation and sentence boundaries. \
        The quality scorer should assign this a high structural bonus. ";

    // ~1 KiB JS function body
    let js_block = |n: usize| -> String {
        format!(
            "<script type=\"text/javascript\">\nfunction processDocument{}(input, options) {{\n  \
             var result = input.trim();\n  if (options.normalize) {{\n    result = \
             result.replace(/\\s+/g, ' ');\n  }}\n  return result;\n}}\n</script>\n",
            n
        )
    };

    // ~512 B CSS block
    let css_block = |n: usize| -> String {
        format!(
            "<style type=\"text/css\">\n.document-container-{n} {{ display: flex; \
             flex-direction: column; padding: 16px; margin: 0 auto; max-width: 960px; }}\n\
             .document-header-{n} {{ font-size: 1.5rem; font-weight: bold; color: #333; }}\n\
             .document-body-{n} {{ line-height: 1.6; color: #444; }}\n</style>\n"
        )
    };

    // Naked JS function chunk (triggers JS_FUNCTION_PATTERN)
    let js_func_chunk = |n: usize| -> String {
        format!(
            "\nfunction renderSection{}(element, data) {{ return element.innerHTML = data; }}\n",
            n
        )
    };

    let mut buf = String::with_capacity(66_000);

    // 10 script blocks each ~1 KiB
    for i in 0..10 {
        buf.push_str(&para.repeat(6)); // ~1.2 KiB prose between blocks
        buf.push_str(&js_block(i));
    }

    // 5 style blocks each ~512 B
    for i in 0..5 {
        buf.push_str(&para.repeat(4));
        buf.push_str(&css_block(i));
    }

    // 3 naked JS function chunks
    for i in 0..3 {
        buf.push_str(&para.repeat(3));
        buf.push_str(&js_func_chunk(i));
    }

    // Pad to ~64 KiB
    while buf.len() < 64 * 1024 {
        buf.push_str(para);
    }
    buf.truncate(64 * 1024);
    buf
}

// ~1 MiB — same ratio as 64 KiB but 16× larger. This is the case that hits the backtracker.
fn corpus_noisy_1mib() -> String {
    let para = "This is a representative paragraph of body text extracted from a web page. \
        It contains normal prose with proper punctuation and sentence boundaries. \
        The quality scorer should assign this a high structural bonus. ";

    let js_block = |n: usize| -> String {
        format!(
            "<script type=\"text/javascript\">\nfunction processDocument{}(input, options) {{\n  \
             var result = input.trim();\n  if (options.normalize) {{\n    result = \
             result.replace(/\\s+/g, ' ');\n  }}\n  return result;\n}}\n</script>\n",
            n
        )
    };

    let css_block = |n: usize| -> String {
        format!(
            "<style type=\"text/css\">\n.document-container-{n} {{ display: flex; \
             flex-direction: column; padding: 16px; margin: 0 auto; max-width: 960px; }}\n\
             .document-header-{n} {{ font-size: 1.5rem; font-weight: bold; color: #333; }}\n\
             .document-body-{n} {{ line-height: 1.6; color: #444; }}\n</style>\n"
        )
    };

    let js_func_chunk = |n: usize| -> String {
        format!(
            "\nfunction renderSection{}(element, data) {{ return element.innerHTML = data; }}\n",
            n
        )
    };

    let mut buf = String::with_capacity(1_100_000);

    // Scale up by 16×: 160 script blocks, 80 style blocks, 48 function chunks
    for i in 0..160 {
        buf.push_str(&para.repeat(6));
        buf.push_str(&js_block(i));
    }
    for i in 0..80 {
        buf.push_str(&para.repeat(4));
        buf.push_str(&css_block(i));
    }
    for i in 0..48 {
        buf.push_str(&para.repeat(3));
        buf.push_str(&js_func_chunk(i));
    }

    while buf.len() < 1024 * 1024 {
        buf.push_str(para);
    }
    buf.truncate(1024 * 1024);
    buf
}

fn bench_quality_clean_1kib(criterion: &mut Criterion) {
    let text = corpus_clean_1kib();
    criterion.bench_function("quality_clean_1kib", |b| {
        b.iter(|| calculate_quality_score(black_box(&text), black_box(None)))
    });
}

fn bench_quality_noisy_64kib(criterion: &mut Criterion) {
    let text = corpus_noisy_64kib();
    criterion.bench_function("quality_noisy_64kib", |b| {
        b.iter(|| calculate_quality_score(black_box(&text), black_box(None)))
    });
}

fn bench_quality_noisy_1mib(criterion: &mut Criterion) {
    let text = corpus_noisy_1mib();
    criterion.bench_function("quality_noisy_1mib", |b| {
        b.iter(|| calculate_quality_score(black_box(&text), black_box(None)))
    });
}

criterion_group!(
    benches,
    bench_quality_clean_1kib,
    bench_quality_noisy_64kib,
    bench_quality_noisy_1mib,
);
criterion_main!(benches);
