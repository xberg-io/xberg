//! Corpus-wide extraction survey: extract all documents and print stats.
//!
//! Replaces `crates/kreuzberg/tests/pdf_markdown_all_docs.rs`.

use crate::Result;
use crate::corpus::{self, CorpusFilter};
use std::path::PathBuf;
use std::time::Instant;

/// Survey configuration.
pub struct SurveyConfig {
    pub fixtures_dir: PathBuf,
    pub file_types: Option<Vec<String>>,
}

/// Stats for one document.
pub struct DocStats {
    pub name: String,
    pub file_type: String,
    pub file_size: u64,
    pub content_length: usize,
    pub heading_count: usize,
    pub table_row_count: usize,
    pub list_item_count: usize,
    pub extraction_ms: f64,
    pub error: Option<String>,
}

/// Run the survey: extract every document and collect stats.
pub async fn run_survey(config: &SurveyConfig) -> Result<Vec<DocStats>> {
    let filter = CorpusFilter {
        file_types: config.file_types.clone(),
        ..Default::default()
    };

    let docs = corpus::build_corpus(&config.fixtures_dir, &filter)?;
    eprintln!("Survey: {} documents", docs.len());

    let extraction_config = kreuzberg::ExtractionConfig {
        output_format: kreuzberg::core::config::OutputFormat::Markdown,
        ..Default::default()
    };

    let mut results = Vec::new();

    let total = docs.len();
    for (idx, doc) in docs.iter().enumerate() {
        eprint!("[{}/{}] {} ...", idx + 1, total, doc.name);
        let t = Instant::now();
        let extraction_future = kreuzberg::core::batch_mode::with_batch_mode(kreuzberg::extract_file(
            &doc.document_path,
            None,
            &extraction_config,
        ));
        let (content, error) = match tokio::time::timeout(std::time::Duration::from_secs(180), extraction_future).await
        {
            Ok(Ok(r)) => (r.content, None),
            Ok(Err(e)) => (String::new(), Some(e.to_string())),
            Err(_) => (String::new(), Some("timeout (180s)".to_string())),
        };
        let extraction_ms = t.elapsed().as_secs_f64() * 1000.0;

        let lines: Vec<&str> = content.lines().collect();
        let heading_count = lines.iter().filter(|l| l.starts_with('#')).count();
        let table_row_count = lines
            .iter()
            .filter(|l| l.starts_with('|') && l.ends_with('|') && !l.contains("---"))
            .count();
        let list_item_count = lines
            .iter()
            .filter(|l| {
                let trimmed = l.trim_start();
                trimmed.starts_with("- ")
                    || trimmed.starts_with("* ")
                    || trimmed.starts_with("+ ")
                    || (trimmed.len() >= 3
                        && trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
                        && trimmed.contains(". "))
            })
            .count();

        eprintln!(" {:.0}ms", extraction_ms);
        results.push(DocStats {
            name: doc.name.clone(),
            file_type: doc.file_type.clone(),
            file_size: doc.file_size,
            content_length: content.len(),
            heading_count,
            table_row_count,
            list_item_count,
            extraction_ms,
            error,
        });
    }

    Ok(results)
}

/// Print survey stats table.
pub fn print_survey_table(results: &[DocStats]) {
    eprintln!(
        "{:<30} {:>6} {:>8} {:>8} {:>5} {:>6} {:>5} {:>8}",
        "Document", "Type", "Size KB", "Content", "Hdgs", "TRows", "Lists", "Time ms"
    );
    eprintln!("{}", "-".repeat(90));

    for s in results {
        let status = if s.error.is_some() { "ERR" } else { "" };
        eprintln!(
            "{:<30} {:>6} {:>8.0} {:>8} {:>5} {:>6} {:>5} {:>7.0} {}",
            if s.name.len() > 29 { &s.name[..29] } else { &s.name },
            s.file_type,
            s.file_size as f64 / 1024.0,
            s.content_length,
            s.heading_count,
            s.table_row_count,
            s.list_item_count,
            s.extraction_ms,
            status,
        );
    }

    // Summary
    let n = results.len();
    let total_time: f64 = results.iter().map(|s| s.extraction_ms).sum();
    let errors = results.iter().filter(|s| s.error.is_some()).count();
    eprintln!("{}", "-".repeat(90));
    eprintln!(
        "Total: {} documents, {:.1}s extraction time, {} errors",
        n,
        total_time / 1000.0,
        errors
    );
}
