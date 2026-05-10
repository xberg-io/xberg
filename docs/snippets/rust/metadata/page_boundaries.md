```rust title="Rust"
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;

    let Some(pages) = &result.metadata.pages else {
        return Ok(());
    };
    let Some(boundaries) = &pages.boundaries else {
        return Ok(());
    };

    for boundary in boundaries.iter().take(3) {
        let page_text = &result.content[boundary.byte_start..boundary.byte_end];
        let preview_end = 100.min(page_text.len());

        println!("Page {}:", boundary.page_number);
        println!("  Byte range: {}-{}", boundary.byte_start, boundary.byte_end);
        println!("  Preview: {}...", &page_text[..preview_end]);
    }

    Ok(())
}
```
