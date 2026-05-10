```rust title="Rust"
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let bytes = tokio::fs::read("document.pdf").await?;

    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name("document.pdf")
        .mime_str("application/pdf")?;
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("chunking", r#"{"max_characters":800,"overlap":100}"#);

    let response = client
        .post("http://localhost:8000/extract")
        .multipart(form)
        .send()
        .await?;

    let result: serde_json::Value = response.error_for_status()?.json().await?;
    if let Some(chunks) = result["chunks"].as_array() {
        println!("{} chunks", chunks.len());
        for chunk in chunks {
            println!("  {} chars", chunk["content"].as_str().unwrap_or("").len());
        }
    }
    Ok(())
}
```
