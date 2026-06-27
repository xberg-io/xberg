```rust title="Rust"
use xberg::{extract, ExtractionConfig, HtmlOutputConfig, HtmlTheme, OutputFormat};

let config = ExtractionConfig {
    output_format: OutputFormat::Html,
    html_output: Some(HtmlOutputConfig {
        theme: HtmlTheme::GitHub,
        ..Default::default()
    }),
    ..Default::default()
};
let result = extract("document.pdf", None, &config).unwrap();
println!("{}", result.content); // HTML with kb-* classes
```
