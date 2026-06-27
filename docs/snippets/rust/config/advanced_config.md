```rust title="Rust"
use xberg::{extract, ExtractionConfig, OcrConfig, ChunkingConfig, LanguageDetectionConfig, TokenReductionConfig, PostProcessorConfig, EmbeddingConfig, EmbeddingModelType};
use xberg::keywords::{KeywordConfig, KeywordAlgorithm};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        use_cache: true,
        enable_quality_processing: true,
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
            output_format: None,
            paddle_ocr_config: None,
            element_config: None,
        }),
        chunking: Some(ChunkingConfig {
            max_characters: 1000,
            overlap: 200,
            embedding: Some(EmbeddingConfig {
                model: EmbeddingModelType::Preset { name: "balanced".to_string() },
                batch_size: 32,
                normalize: true,
                show_download_progress: false,
                cache_dir: None,
            }),
            ..Default::default()
        }),
        language_detection: Some(LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.8,
            detect_multiple: false,
        }),
        keywords: Some(KeywordConfig {
            algorithm: KeywordAlgorithm::Yake,
            max_keywords: 10,
            min_score: 0.1,
            ngram_range: (1, 3),
            language: Some("en".to_string()),
            ..Default::default()
        }),
        token_reduction: Some(TokenReductionConfig {
            mode: "moderate".to_string(),
            preserve_important_words: true,
        }),
        postprocessor: Some(PostProcessorConfig {
            enabled: true,
            enabled_processors: None,
            disabled_processors: None,
        }),
        ..Default::default()
    };

    let result = extract("document.pdf", None::<&str>, &config).await?;
    println!("Content: {}", result.content);
    if let Some(langs) = &result.detected_languages {
        println!("Languages: {:?}", langs);
    }
    println!("Chunks: {}", result.chunks.as_ref().map(|c| c.len()).unwrap_or(0));
    Ok(())
}
```
