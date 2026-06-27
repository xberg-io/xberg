Use Xberg::{extract_sync, ExtractionConfig, ChunkingConfig, PageConfig};

Let config = ExtractionConfig {
chunking: Some(ChunkingConfig {
max_characters: 500,
overlap: 50,
..Default::default()
}),
pages: Some(PageConfig {
extract_pages: true,
..Default::default()
}),
..Default::default()
};

Let result = extract_sync("document.pdf", None, &config)?;

If let Some(chunks) = result.chunks {
for chunk in chunks {
if let (Some(first), Some(last)) = (chunk.metadata.first_page, chunk.metadata.last_page) {
let page_range = if first == last {
format!("Page {}", first)
} else {
format!("Pages {}-{}", first, last)
};
println!("Chunk: {}... ({})", chunk.content.chars().take(50).collect::<String>(), page_range);
}
}
}
