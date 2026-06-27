```rust title="Rust"
use xberg::extract_sync;

fn main() -> xberg::Result<()> {
    let result = extract_sync("document.pdf", None, &Default::default())?;
    println!("{}", result.content);
    Ok(())
}
```
