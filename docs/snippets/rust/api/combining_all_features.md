```rust title="Rust"
use kreuzberg::{
    ChunkingConfig, ChunkerType, ExtractionConfig, ImageExtractionConfig,
    OcrConfig, OutputFormat, extract_file_sync,
};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        // OCR: force Tesseract on all pages with English text
        force_ocr: false,
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            ..Default::default()
        }),
        // Chunking: semantic markdown chunks of ~800 chars, 100-char overlap
        chunking: Some(ChunkingConfig {
            max_characters: 800,
            overlap: 100,
            chunker_type: ChunkerType::Markdown,
            prepend_heading_context: true,
            ..Default::default()
        }),
        // Output: include document structure and tables
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        // Images: extract embedded images
        images: Some(ImageExtractionConfig {
            extract_images: true,
            ..Default::default()
        }),
        // Cache extracted results on disk
        use_cache: true,
        enable_quality_processing: true,
        ..Default::default()
    };

    let result = extract_file_sync("report.pdf", None, &config)?;

    println!("Content ({} chars):", result.content.len());
    println!("{}", &result.content[..result.content.len().min(200)]);

    if let Some(chunks) = &result.chunks {
        println!("\nChunks: {}", chunks.len());
    }
    println!("Tables: {}", result.tables.len());
    if let Some(langs) = &result.detected_languages {
        println!("Languages: {:?}", langs);
    }
    if let Some(method) = result.extraction_method {
        println!("Extraction method: {:?}", method);
    }
    Ok(())
}
```
