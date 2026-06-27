```rust title="Rust"
use std::fs;
use xberg::extract;

fn main() -> xberg::Result<()> {
    let data = fs::read("document.pdf")?;
    let result = extract(&data, "application/pdf", &Default::default())?;

    println!("{}", result.content);
    println!("Success: true");
    println!("Content length: {} characters", result.content.len());
    Ok(())
}
```
