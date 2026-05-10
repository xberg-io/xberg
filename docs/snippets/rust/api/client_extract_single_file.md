```rust title="Rust"
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let bytes = tokio::fs::read("document.pdf").await?;
    let file_name = Path::new("document.pdf")
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.pdf");

    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name.to_string())
        .mime_str("application/pdf")?;
    let form = reqwest::multipart::Form::new().part("file", part);

    let response = client
        .post("http://localhost:8000/extract")
        .multipart(form)
        .send()
        .await?;

    let result: serde_json::Value = response.error_for_status()?.json().await?;
    println!("{}", result["content"].as_str().unwrap_or(""));
    Ok(())
}
```
