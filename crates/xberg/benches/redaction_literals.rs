use std::collections::HashSet;
use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use xberg::ExtractedDocument;
use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::text::redaction::redact;
use xberg::types::redaction::PiiCategory;

fn bench_literal_redaction(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("benchmark runtime");
    let mut group = criterion.benchmark_group("literal_redaction");

    for count in [4, 32, 256] {
        let config = RedactionConfig {
            categories: HashSet::from([PiiCategory::Custom("literal".into())]),
            custom_terms: (0..count)
                .map(|index| RedactionTerm::literal(format!("Project-{index:04}")))
                .collect(),
            ..Default::default()
        };
        let input = "Draft text for project-0001 and Project-0003. Public material.\n".repeat(128);
        let make_document = || {
            let mut document = ExtractedDocument::default();
            document.content = input.clone();
            document
        };
        let mut probe = make_document();
        runtime
            .block_on(redact(&mut probe, &config))
            .expect("redact benchmark probe");
        assert_eq!(probe.redaction_report.expect("redaction report").total_redacted, 256);

        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &config, |bencher, config| {
            bencher.iter_batched(
                make_document,
                |mut document| {
                    runtime
                        .block_on(redact(&mut document, black_box(config)))
                        .expect("redact benchmark input");
                    black_box(document);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, bench_literal_redaction);
criterion_main!(benches);
