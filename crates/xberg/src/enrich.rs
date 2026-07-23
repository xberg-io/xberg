//! Unified enrichment chokepoint composing captioning, NER, classification,
//! and (future) transcription on top of an [`ExtractedDocument`].
//!
//! # Design
//!
//! Each enrichment stage is independently optional and feature-gated. Passing
//! `None` for a stage's config field skips that stage entirely. Stages run
//! sequentially so that later stages can see prior results without complex
//! dependency tracking.
//!
//! ## Stage order
//!
//! 1. Classification — operates on the full document text (`content`)
//! 2. Chunk classification — multi-labels each entry of `ExtractedDocument::chunks`
//!    in place; a no-op when the document has no chunks
//! 3. NER — operates on the full document text (`content`)
//! 4. Captioning — operates on images extracted into `ExtractedDocument::images`
//!
//! Transcription is reserved for a future backend and is kept present in the
//! config surface so callers can wire it today; any attempt to activate it
//! returns an explicit not-yet-implemented error.
//!
//! # Example
//!
//! ```ignore
//! use xberg::{ExtractInput, ExtractionConfig, extract, enrich, EnrichmentConfig};
//!
//! # async fn run() -> xberg::Result<()> {
//! let output = extract(ExtractInput::from_uri("document.pdf"), &ExtractionConfig::default()).await?;
//! let extraction = output.results.into_iter().next().expect("one input yields one result");
//! let config = EnrichmentConfig::default();
//! let enriched = enrich(extraction, &config).await?;
//! assert!(enriched.entities.is_none()); // no NER config supplied
//! # Ok(())
//! # }
//! ```

use crate::types::ExtractedDocument;

#[cfg(feature = "ner")]
use std::sync::Arc;

#[cfg(feature = "classification")]
use crate::ClassificationLabel;

#[cfg(feature = "ner")]
use crate::types::entity::{Entity, EntityCategory};

/// NER enrichment knob: which backend to use and which categories to request.
#[cfg(feature = "ner")]
pub struct NerEnrichmentConfig {
    /// The NER backend implementation. Wrap a concrete backend in `Arc` and
    /// assign it here:
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use xberg::{LlmBackend, LlmConfig, enrich::NerEnrichmentConfig};
    ///
    /// let config = NerEnrichmentConfig {
    ///     backend: Arc::new(LlmBackend::new(LlmConfig::default())),
    ///     categories: vec![],
    /// };
    /// ```
    pub backend: Arc<dyn crate::text::ner::NerBackend>,
    /// Entity categories to detect. An empty slice tells the backend to return
    /// every category it recognises.
    pub categories: Vec<EntityCategory>,
}

/// Classification enrichment knob: how to label the document.
#[cfg(feature = "classification")]
pub struct ClassificationEnrichmentConfig {
    /// Label set and LLM settings for the classification stage.
    pub config: crate::core::config::PageClassificationConfig,
}

/// Chunk-classification enrichment knob: how to multi-label individual chunks.
///
/// Operates on `ExtractedDocument::chunks` in place — the caller must have
/// already produced chunks (e.g. via `ExtractionConfig::chunking`) for this
/// stage to have any effect; a document with no chunks is a no-op.
#[cfg(feature = "classification")]
pub struct ChunkClassificationEnrichmentConfig {
    /// Label-definition set and LLM/batching settings for the chunk-classification stage.
    pub config: crate::core::config::ChunkClassificationConfig,
}

/// Captioning enrichment knob: which LLM to use for image captions.
///
/// The enrichment stage calls [`crate::captioning::caption_image`] for every
/// image in `ExtractedDocument::images` that has non-empty `data`. Images with
/// empty byte data (e.g. reference-only images populated via `source_path`) are
/// skipped rather than forwarded to the VLM.
#[cfg(feature = "captioning")]
pub struct CaptioningEnrichmentConfig {
    /// LLM / VLM configuration forwarded verbatim to each `caption_image` call.
    pub config: crate::core::config::LlmConfig,
    /// Optional custom prompt override forwarded to every `caption_image` call.
    /// `None` uses the default `RegionKind::Caption` prompt.
    pub custom_prompt: Option<String>,
}

/// Aggregated enrichment configuration.
///
/// Each field is feature-gated and independently optional. Set a field to
/// `Some(...)` to activate the corresponding stage; leave it `None` to skip.
///
/// `EnrichmentConfig::default()` produces a no-op config: all stages skipped,
/// and `enrich` returns an `EnrichedResult` with all enrichment fields `None`.
#[derive(Default)]
pub struct EnrichmentConfig {
    /// NER stage.  `None` skips entity detection.
    #[cfg(feature = "ner")]
    pub ner: Option<NerEnrichmentConfig>,

    /// Document-classification stage.  `None` skips classification.
    #[cfg(feature = "classification")]
    pub classification: Option<ClassificationEnrichmentConfig>,

    /// Chunk-classification stage.  `None` skips per-chunk multi-label classification.
    #[cfg(feature = "classification")]
    pub chunk_classification: Option<ChunkClassificationEnrichmentConfig>,

    /// Image-captioning stage.  `None` skips captioning.
    #[cfg(feature = "captioning")]
    pub captioning: Option<CaptioningEnrichmentConfig>,

    /// Transcription stage (reserved — not yet implemented).
    ///
    /// Any `Some(...)` value causes `enrich` to return
    /// `Err(XbergError::Other("transcription backend not yet implemented"))`.
    /// Include config here now so call-sites compile and activate the stage once
    /// the backend lands.
    #[cfg(feature = "transcription-types")]
    pub transcription: Option<crate::core::config::TranscriptionConfig>,
}

/// Extraction result with optional enrichment layers applied.
///
/// The `extraction` field carries the original [`ExtractedDocument`] unchanged.
/// Enrichment fields are `None` when the corresponding stage was not configured
/// or when the feature was compiled out.
pub struct EnrichedResult {
    /// The original extraction result, unchanged by the enrichment pipeline.
    pub extraction: ExtractedDocument,

    /// Detected named entities (populated by the NER stage).
    #[cfg(feature = "ner")]
    pub entities: Option<Vec<Entity>>,

    /// Document-level classification labels (populated by the classification stage).
    ///
    /// `classify_document` aggregates across all pages; the result here is the
    /// post-aggregation label set (one entry for single-label mode, any subset
    /// of the configured labels for multi-label mode).
    #[cfg(feature = "classification")]
    pub classification: Option<Vec<ClassificationLabel>>,

    /// Per-image captions indexed parallel to `extraction.images`
    /// (populated by the captioning stage).
    ///
    /// `captions[i]` is the caption for `extraction.images.as_deref().unwrap()[i]`.
    /// Images whose `data` bytes were empty produce an empty string rather than a
    /// VLM call.
    #[cfg(feature = "captioning")]
    pub captions: Option<Vec<String>>,
}

/// Apply enrichment stages to an extraction result.
///
/// Stages run sequentially: classification → NER → captioning.
/// On any error the partial result is dropped and the error is returned
/// immediately.
///
/// # Example
///
/// ```ignore
/// use xberg::{ExtractInput, ExtractionConfig, extract, enrich, EnrichmentConfig};
///
/// # async fn run() -> xberg::Result<()> {
/// let output = extract(ExtractInput::from_uri("report.pdf"), &ExtractionConfig::default()).await?;
/// let extraction = output.results.into_iter().next().expect("one input yields one result");
///
/// // Skip all stages — identity pass.
/// let enriched = enrich(extraction, &EnrichmentConfig::default()).await?;
/// assert!(enriched.entities.is_none());
/// assert!(enriched.classification.is_none());
/// assert!(enriched.captions.is_none());
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// - [`crate::XbergError::Validation`] when the classification config has an
///   empty label set (propagated from `classify_document`).
/// - [`crate::XbergError::Other`] when the NER or captioning backends fail.
/// - [`crate::XbergError::Other`] when `config.transcription` is `Some`:
///   the transcription backend is not yet implemented.
#[cfg_attr(alef, alef(skip))]
#[cfg_attr(not(feature = "classification"), allow(unused_mut))]
pub async fn enrich(mut extraction: ExtractedDocument, config: &EnrichmentConfig) -> crate::Result<EnrichedResult> {
    // read inside `#[cfg(...)]` branches that are all compiled out — silence
    #[cfg(not(any(
        feature = "transcription-types",
        feature = "classification",
        feature = "ner",
        feature = "captioning",
    )))]
    let _ = config;

    #[cfg(feature = "transcription-types")]
    if config.transcription.is_some() {
        return Err(crate::XbergError::Other(
            "transcription backend not yet implemented; set config.transcription = None to skip".into(),
        ));
    }

    #[cfg(feature = "classification")]
    let classification = if let Some(ref cfg) = config.classification {
        let pages: Vec<&str> = match extraction.pages.as_deref() {
            Some(pages) => pages.iter().map(|p| p.content.as_str()).collect(),
            None => vec![extraction.content.as_str()],
        };
        Some(crate::text::classification::classify_document(&pages, &cfg.config).await?)
    } else {
        None
    };

    #[cfg(feature = "classification")]
    if let Some(ref cfg) = config.chunk_classification {
        crate::text::classification::classify_chunks(&mut extraction, &cfg.config).await?;
    }

    #[cfg(feature = "ner")]
    let entities = if let Some(ref cfg) = config.ner {
        Some(crate::text::ner::detect_entities(&extraction.content, cfg.backend.as_ref(), &cfg.categories).await?)
    } else {
        None
    };

    #[cfg(feature = "captioning")]
    let captions = if let Some(ref cfg) = config.captioning {
        match extraction.images.as_deref() {
            None | Some([]) => Some(Vec::new()),
            Some(images) => {
                let mut out = Vec::with_capacity(images.len());
                for image in images {
                    let caption = if image.data.is_empty() {
                        String::new()
                    } else {
                        crate::captioning::caption_image(&image.data, &cfg.config, cfg.custom_prompt.as_deref()).await?
                    };
                    out.push(caption);
                }
                Some(out)
            }
        }
    } else {
        None
    };

    Ok(EnrichedResult {
        extraction,
        #[cfg(feature = "ner")]
        entities,
        #[cfg(feature = "classification")]
        classification,
        #[cfg(feature = "captioning")]
        captions,
    })
}
