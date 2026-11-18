use std::collections::HashSet;
use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use xberg::ExtractedDocument;
use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::text::redaction::redact;
use xberg::types::redaction::PiiCategory;

const LINE_REPEATS: usize = 128;
const LARGE_DICTIONARIES: &[usize] = &[32, 256];
const BASE_LINE: &str = "Draft text for project-0001 and Project-0003. Public material.\n";

struct Workload {
    name: &'static str,
    opener: &'static str,
    first_term: &'static str,
    first_hit: &'static str,
    case_sensitive: bool,
    counts: &'static [usize],
}

const WORKLOADS: &[Workload] = &[
    Workload {
        name: "ascii-case-insensitive",
        opener: "Draft",
        first_term: "Project-0001",
        first_hit: "project-0001",
        case_sensitive: false,
        counts: &[4, 32, 256],
    },
    Workload {
        name: "non-ascii-text-case-insensitive-fallback",
        opener: "Café",
        first_term: "Project-0001",
        first_hit: "project-0001",
        case_sensitive: false,
        counts: LARGE_DICTIONARIES,
    },
    Workload {
        name: "self-overlapping-term-case-insensitive-fallback",
        opener: "Draft",
        first_term: "Anna-Brianna",
        first_hit: "anna-brianna",
        case_sensitive: false,
        counts: LARGE_DICTIONARIES,
    },
    Workload {
        name: "ascii-case-sensitive-shared",
        opener: "Draft",
        first_term: "Project-0001",
        first_hit: "Project-0001",
        case_sensitive: true,
        counts: LARGE_DICTIONARIES,
    },
];

impl Workload {
    fn line(&self) -> String {
        format!(
            "{} text for {} and Project-0003. Public material.\n",
            self.opener, self.first_hit,
        )
    }

    fn terms(&self, count: usize) -> Vec<RedactionTerm> {
        (0..count)
            .map(|index| {
                let value = if index == 1 {
                    self.first_term.to_string()
                } else {
                    format!("Project-{index:04}")
                };
                let mut term = RedactionTerm::literal(value);
                term.case_sensitive = self.case_sensitive;
                term
            })
            .collect()
    }
}

fn bench_literal_redaction(criterion: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("benchmark runtime");
    let mut group = criterion.benchmark_group("literal_redaction");

    for workload in WORKLOADS {
        let line = workload.line();
        assert_eq!(line.len(), BASE_LINE.len(), "input byte count for {}", workload.name);
        let input = line.repeat(LINE_REPEATS);
        let make_document = || {
            let mut document = ExtractedDocument::default();
            document.content = input.clone();
            document
        };
        group.throughput(Throughput::Bytes(input.len() as u64));

        for &count in workload.counts {
            let config = RedactionConfig {
                categories: HashSet::from([PiiCategory::Custom("literal".into())]),
                custom_terms: workload.terms(count),
                ..Default::default()
            };
            let mut probe = make_document();
            runtime
                .block_on(redact(&mut probe, &config))
                .expect("redact benchmark probe");
            assert_eq!(
                probe.redaction_report.expect("redaction report").total_redacted,
                (2 * LINE_REPEATS) as u32,
                "redaction count for {}/{count}",
                workload.name,
            );

            group.bench_with_input(BenchmarkId::new(workload.name, count), &config, |bencher, config| {
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
    }
    group.finish();
}

criterion_group!(benches, bench_literal_redaction);
criterion_main!(benches);
