Use Xberg::{extract_sync, ExtractionConfig, PageConfig};

Let config = ExtractionConfig {
pages: Some(PageConfig {
extract_pages: true,
..Default::default()
}),
..Default::default()
};

Let result = extract_sync("document.pdf", &config)?;

If let Some(pages) = result.pages {
for page in pages {
println!("Page {}:", page.page_number);
println!(" Content: {} chars", page.content.len());
println!(" Tables: {}", page.tables.len());
println!(" Images: {}", page.images.len());
}
}
