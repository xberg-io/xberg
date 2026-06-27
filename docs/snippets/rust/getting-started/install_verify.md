```rust title="Rust"
use xberg::extract;

fn main() -> xberg::Result<()> {
    let result = extract("document.pdf", None, &Default::default())?;
    println!("Extraction successful: {}", !result.content.is_empty());
    println!("Content length: {} characters", result.content.len());
    Ok(())
}
```
