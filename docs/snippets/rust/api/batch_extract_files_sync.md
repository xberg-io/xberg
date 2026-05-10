```rust title="Rust"
use kreuzberg::{batch_extract_files_sync, BatchFileItem, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let items = vec![
        BatchFileItem { path: "doc1.pdf".into(), config: None },
        BatchFileItem { path: "doc2.docx".into(), config: None },
        BatchFileItem { path: "report.pdf".into(), config: None },
    ];
    let results = batch_extract_files_sync(items, &config)?;

    for (i, result) in results.iter().enumerate() {
        println!("File {}: {} chars", i, result.content.len());
    }
    Ok(())
}
```
