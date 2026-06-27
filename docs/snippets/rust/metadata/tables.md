```rust title="Rust"
use xberg::{extract_sync, ExtractionConfig};

fn main() -> xberg::Result<()> {
    let result = extract_sync("document.pdf", None, &ExtractionConfig::default())?;

    for table in &result.tables {
        println!("Table with {} rows", table.cells.len());
        println!("{}", table.markdown);

        for row in &table.cells {
            println!("{:?}", row);
        }
    }
    Ok(())
}
```
