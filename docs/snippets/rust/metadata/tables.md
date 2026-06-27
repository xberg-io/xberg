```rust title="Rust"
use xberg::{extract, ExtractionConfig};

fn main() -> xberg::Result<()> {
    let result = extract("document.pdf", None, &ExtractionConfig::default())?;

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
