```rust title="Rust"
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let result = extract_file_sync("document.pdf", None, &ExtractionConfig::default())?;

    if let Some(pdf_meta) = result.metadata.pdf {
        if let Some(pages) = pdf_meta.page_count {
            println!("Pages: {}", pages);
        }
        if let Some(author) = pdf_meta.author {
            println!("Author: {}", author);
        }
        if let Some(title) = pdf_meta.title {
            println!("Title: {}", title);
        }
    }

    let html_result = extract_file_sync("page.html", None, &ExtractionConfig::default())?;
    if let Some(html_meta) = html_result.metadata.html {
        if let Some(title) = html_meta.title {
            println!("Title: {}", title);
        }
        if let Some(desc) = html_meta.description {
            println!("Description: {}", desc);
        }

        // Access keywords array
        println!("Keywords: {:?}", html_meta.keywords);

        // Access canonical URL (renamed from canonical)
        if let Some(canonical) = html_meta.canonical_url {
            println!("Canonical URL: {}", canonical);
        }

        // Access Open Graph fields as a map
        if let Some(og_image) = html_meta.open_graph.get("image") {
            println!("Open Graph Image: {}", og_image);
        }
        if let Some(og_title) = html_meta.open_graph.get("title") {
            println!("Open Graph Title: {}", og_title);
        }

        // Access Twitter Card fields as a map
        if let Some(twitter_card) = html_meta.twitter_card.get("card") {
            println!("Twitter Card Type: {}", twitter_card);
        }

        // Access new fields
        if let Some(lang) = html_meta.language {
            println!("Language: {}", lang);
        }

        // Access headers
        if !html_meta.headers.is_empty() {
            for header in &html_meta.headers {
                println!("Header (level {}): {}", header.level, header.text);
            }
        }

        // Access links
        if !html_meta.links.is_empty() {
            for link in &html_meta.links {
                println!("Link: {} ({})", link.href, link.text);
            }
        }

        // Access images
        if !html_meta.images.is_empty() {
            for image in &html_meta.images {
                println!("Image: {}", image.src);
            }
        }

        // Access structured data
        if !html_meta.structured_data.is_empty() {
            println!("Structured data items: {}", html_meta.structured_data.len());
        }
    }
    Ok(())
}
```
