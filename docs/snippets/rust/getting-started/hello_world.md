```rust title="Rust"
use xberg::extract;

fn main() -> xberg::Result<()> {
    let result = extract("document.pdf", None, &Default::default())?;
    println!("{}", result.content);
    Ok(())
}
```
