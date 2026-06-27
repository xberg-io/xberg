```rust title="Rust"
use xberg::{extract_sync, ExtractionConfig, PdfConfig, HierarchyConfig};

fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            hierarchy: Some(HierarchyConfig {
                enabled: true,
                detection_threshold: Some(0.75),
                ocr_coverage_threshold: Some(0.8),
                min_level: Some(1),
                max_level: Some(5),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_sync("document.pdf", None::<&str>, &config)?;
    println!("Hierarchy levels: {}", result.hierarchy.len());
    Ok(())
}
```
