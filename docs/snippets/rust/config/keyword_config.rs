use xberg::{extract, ExtractionConfig};
use xberg::keywords::{KeywordConfig, KeywordAlgorithm, YakeParams, RakeParams};

// Example 1: Basic YAKE configuration
// Uses YAKE algorithm with default parameters and English stopword filtering
pub async fn basic_yake() -> xberg::Result<()> {
    let config = ExtractionConfig {
        keywords: Some(KeywordConfig {
            algorithm: KeywordAlgorithm::Yake,
            max_keywords: 10,
            min_score: 0.0,
            ngram_range: (1, 3),
            language: Some("en".to_string()),
            yake_params: None,
            rake_params: None,
        }),
        ..Default::default()
    };

    let result = extract("document.pdf", None::<&str>, &config).await?;
    println!("Keywords: {:?}", result.keywords);
    Ok(())
}

// Example 2: Advanced YAKE with custom parameters
// Fine-tunes YAKE with custom window size for co-occurrence analysis
#[cfg(feature = "keywords-yake")]
pub async fn advanced_yake() -> xberg::Result<()> {
    let config = ExtractionConfig {
        keywords: Some(KeywordConfig {
            algorithm: KeywordAlgorithm::Yake,
            max_keywords: 15,
            min_score: 0.1,
            ngram_range: (1, 2),
            language: Some("en".to_string()),
            yake_params: Some(YakeParams {
                window_size: 1,
            }),
            rake_params: None,
        }),
        ..Default::default()
    };

    let result = extract("document.pdf", None::<&str>, &config).await?;
    println!("Keywords: {:?}", result.keywords);
    Ok(())
}

// Example 3: RAKE configuration
// Uses RAKE algorithm for rapid keyword extraction with phrase constraints
#[cfg(feature = "keywords-rake")]
pub async fn rake_config() -> xberg::Result<()> {
    let config = ExtractionConfig {
        keywords: Some(KeywordConfig {
            algorithm: KeywordAlgorithm::Rake,
            max_keywords: 10,
            min_score: 5.0,
            ngram_range: (1, 3),
            language: Some("en".to_string()),
            yake_params: None,
            rake_params: Some(RakeParams {
                min_word_length: 1,
                max_words_per_phrase: 3,
            }),
        }),
        ..Default::default()
    };

    let result = extract("document.pdf", None::<&str>, &config).await?;
    println!("Keywords: {:?}", result.keywords);
    Ok(())
}
