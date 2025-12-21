use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use kreuzberg::text::token_reduction::{ReductionLevel, TokenReducer, TokenReductionConfig};
use std::hint::black_box;

/// Generate test text of approximately the specified size in bytes.
fn generate_text(size_bytes: usize) -> String {
    let base_text = "The quick brown fox jumps over the lazy dog. This is a test sentence with various words. \
                      Machine learning and artificial intelligence are fascinating topics. \
                      Data science requires understanding statistics and programming. \
                      Natural language processing involves tokenization and filtering. \
                      This text contains repeated words like test and words and more words. ";

    let repetitions = (size_bytes / base_text.len()) + 1;
    let text = base_text.repeat(repetitions);
    text.chars().take(size_bytes).collect()
}

/// CJK test text approximately 100 bytes.
fn generate_cjk_text() -> String {
    "这是一个关于机器学习和人工智能的测试文本。\
     数据科学需要理解统计和编程。\
     自然语言处理涉及分词和过滤。\
     这个文本包含重复的词汇和更多词汇。"
        .repeat(3)
}

fn benchmark_light_reduction(c: &mut Criterion) {
    let config = TokenReductionConfig {
        level: ReductionLevel::Light,
        use_simd: false,
        ..Default::default()
    };
    let reducer = TokenReducer::new(&config, None).unwrap();

    let mut group = c.benchmark_group("light_reduction");
    group.sample_size(100);

    for size in [1024, 102400, 1048576].iter() {
        let text = generate_text(*size);
        group.bench_with_input(BenchmarkId::from_parameter(format!("{}B", size)), size, |b, _| {
            b.iter(|| reducer.reduce(black_box(&text)));
        });
    }

    group.finish();
}

fn benchmark_moderate_reduction(c: &mut Criterion) {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };
    let reducer = TokenReducer::new(&config, None).unwrap();

    let mut group = c.benchmark_group("moderate_reduction");
    group.sample_size(100);

    for size in [1024, 102400, 1048576].iter() {
        let text = generate_text(*size);
        group.bench_with_input(BenchmarkId::from_parameter(format!("{}B", size)), size, |b, _| {
            b.iter(|| reducer.reduce(black_box(&text)));
        });
    }

    group.finish();
}

fn benchmark_aggressive_reduction(c: &mut Criterion) {
    let config = TokenReductionConfig {
        level: ReductionLevel::Aggressive,
        use_simd: false,
        ..Default::default()
    };
    let reducer = TokenReducer::new(&config, None).unwrap();

    let mut group = c.benchmark_group("aggressive_reduction");
    group.sample_size(50);

    for size in [1024, 102400, 1048576].iter() {
        let text = generate_text(*size);
        group.bench_with_input(BenchmarkId::from_parameter(format!("{}B", size)), size, |b, _| {
            b.iter(|| reducer.reduce(black_box(&text)));
        });
    }

    group.finish();
}

fn benchmark_cjk_text(c: &mut Criterion) {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };
    let reducer = TokenReducer::new(&config, Some("zh")).unwrap();

    let text = generate_cjk_text();

    c.bench_function("cjk_moderate_reduction", |b| {
        b.iter(|| reducer.reduce(black_box(&text)));
    });
}

fn benchmark_batch_reduction(c: &mut Criterion) {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        enable_parallel: false,
        ..Default::default()
    };
    let reducer = TokenReducer::new(&config, None).unwrap();

    let texts: Vec<String> = (0..10)
        .map(|i| {
            let size = 10240 * (i + 1);
            generate_text(size)
        })
        .collect();

    let text_refs: Vec<&str> = texts.iter().map(|t| t.as_str()).collect();

    c.bench_function("batch_reduction_10_texts", |b| {
        b.iter(|| reducer.batch_reduce(black_box(&text_refs)));
    });
}

criterion_group!(
    benches,
    benchmark_light_reduction,
    benchmark_moderate_reduction,
    benchmark_aggressive_reduction,
    benchmark_cjk_text,
    benchmark_batch_reduction
);
criterion_main!(benches);
