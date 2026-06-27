//! OpenAPI 3.1 schema generation for Xberg API.
//!
//! This module generates OpenAPI documentation from Rust types using utoipa.
//! The schema is available at the `/openapi.json` endpoint.

#[cfg(feature = "api")]
use utoipa::OpenApi;

/// OpenAPI documentation structure.
///
/// Defines all endpoints, request/response schemas, and examples
/// for the Xberg document extraction API.
#[cfg(feature = "api")]
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Xberg API",
        version = env!("CARGO_PKG_VERSION"),
        description = "High-performance document intelligence API for extracting text, metadata, and structured data from PDFs, Office documents, images, and 96 formats.",
        contact(
            name = "Xberg",
            url = "https://xberg.io"
        ),
        license(
            name = "Apache-2.0 OR MIT"
        )
    ),
    servers(
        (url = "http://localhost:8000", description = "Local development server"),
        (url = "https://api.xberg.io", description = "Production server (example)")
    ),
    paths(
        crate::api::handlers::health_handler,
        crate::api::handlers::info_handler,
        crate::api::handlers::extract_handler,
        crate::api::handlers::extract_structured_handler,
        crate::api::handlers::detect_handler,
        crate::api::handlers::formats_handler,
        crate::api::handlers::cache_stats_handler,
        crate::api::handlers::cache_clear_handler,
        crate::api::handlers::cache_manifest_handler,
        crate::api::handlers::cache_warm_handler,
        crate::api::handlers::embed_handler,
        crate::api::handlers::rerank_handler,
        crate::api::handlers::chunk_handler,
        crate::api::handlers::version_handler,
        crate::api::openweb::openweb_external_handler,
        crate::api::openweb::openweb_docling_handler,
    ),
    components(
        schemas(
            crate::api::types::HealthResponse,
            crate::api::types::PluginStatus,
            crate::api::types::InfoResponse,
            crate::api::types::ErrorResponse,
            crate::api::types::CacheStatsResponse,
            crate::api::types::CacheClearResponse,
            crate::api::types::EmbedRequest,
            crate::api::types::EmbedResponse,
            crate::api::types::RerankRequest,
            crate::api::types::RerankResponse,
            crate::RerankedDocument,
            crate::api::types::ChunkRequest,
            crate::api::types::ChunkResponse,
            crate::api::types::ChunkItem,
            crate::api::types::ChunkingConfigRequest,
            crate::api::types::ChunkingConfigResponse,
            crate::api::types::VersionResponse,
            crate::api::types::DetectResponse,
            crate::api::types::ManifestResponse,
            crate::api::types::ManifestEntryResponse,
            crate::api::types::WarmRequest,
            crate::api::types::WarmResponse,
            crate::core::mime::SupportedFormat,
            crate::core::config::ExtractionOutput,
            crate::core::config::ExtractionSummary,
            crate::core::config::ExtractionErrorItem,
            crate::types::extraction::ExtractionResult,
            crate::types::extraction::Chunk,
            crate::types::extraction::ChunkMetadata,
            crate::types::extraction::ExtractedImage,
            crate::types::extraction::Element,
            crate::types::extraction::ElementMetadata,
            crate::types::extraction::ElementId,
            crate::types::extraction::ElementType,
            crate::types::extraction::BoundingBox,
            crate::types::ocr_elements::OcrElement,
            crate::types::ocr_elements::OcrBoundingGeometry,
            crate::types::ocr_elements::OcrConfidence,
            crate::types::ocr_elements::OcrRotation,
            crate::types::ocr_elements::OcrElementLevel,
            crate::types::ocr_elements::OcrElementConfig,
            crate::types::metadata::Metadata,
            crate::types::tables::Table,
            crate::types::page::PageContent,
            crate::types::djot::DjotContent,
            crate::api::types::OpenWebDocumentResponse,
            crate::api::types::OpenWebDocumentMetadata,
            crate::api::types::DoclingCompatResponse,
            crate::api::types::DoclingCompatDocument,
            crate::api::types::StructuredExtractionResponse,
        )
    ),
    tags(
        (name = "health", description = "Health and status endpoints"),
        (name = "extraction", description = "Document extraction endpoints"),
        (name = "cache", description = "Cache management endpoints"),
        (name = "embeddings", description = "Text embedding generation"),
        (name = "reranking", description = "Document reranking by relevance score"),
        (name = "chunking", description = "Text chunking operations"),
        (name = "openweb", description = "OpenWebUI compatibility endpoints")
    )
)]
#[cfg_attr(alef, alef(skip))]
pub struct ApiDoc;

/// Generate OpenAPI JSON schema.
///
/// Returns the complete OpenAPI 3.1 specification as a JSON string.
///
/// # Examples
///
/// ```no_run
/// use xberg::api::openapi::openapi_json;
///
/// let schema = openapi_json();
/// println!("{}", schema);
/// ```
#[cfg(feature = "api")]
#[cfg_attr(alef, alef(skip))]
pub fn openapi_json() -> String {
    ApiDoc::openapi().to_pretty_json().unwrap_or_else(|_| "{}".to_string())
}

#[cfg(not(feature = "api"))]
#[cfg_attr(alef, alef(skip))]
pub(crate) fn openapi_json() -> String {
    r#"{"error": "API feature not enabled"}"#.to_string()
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "api")]
    use super::*;

    #[test]
    #[cfg(feature = "api")]
    fn test_openapi_schema_generation() {
        let schema = openapi_json();
        assert!(!schema.is_empty());
        assert!(schema.contains("Xberg API"));
        assert!(schema.contains("/health"));
        assert!(schema.contains("/extract"));
    }

    #[test]
    #[cfg(feature = "api")]
    fn test_openapi_schema_valid_json() {
        let schema = openapi_json();
        let parsed: serde_json::Value = serde_json::from_str(&schema).expect("Invalid JSON");
        assert!(parsed.is_object());
        assert!(parsed["openapi"].is_string());
    }

    #[test]
    #[cfg(feature = "api")]
    fn test_openapi_includes_all_endpoints() {
        let schema = openapi_json();
        // Health endpoints
        assert!(schema.contains("/health"));
        assert!(schema.contains("/info"));
        assert!(schema.contains("/version"));
        // Extraction
        assert!(schema.contains("/extract"));
        assert!(schema.contains("/detect"));
        // Cache
        assert!(schema.contains("/cache/stats"));
        assert!(schema.contains("/cache/clear"));
        assert!(schema.contains("/cache/manifest"));
        assert!(schema.contains("/cache/warm"));
        // Embeddings
        assert!(schema.contains("/embed"));
        // Reranking
        assert!(schema.contains("/rerank"));
        // Chunking
        assert!(schema.contains("/chunk"));
    }

    #[test]
    #[cfg(feature = "api")]
    fn test_openapi_includes_schemas() {
        let schema = openapi_json();
        assert!(schema.contains("HealthResponse"));
        assert!(schema.contains("ErrorResponse"));
        assert!(schema.contains("EmbedRequest"));
        assert!(schema.contains("RerankRequest"));
        assert!(schema.contains("ChunkRequest"));
        assert!(schema.contains("VersionResponse"));
        assert!(schema.contains("DetectResponse"));
        assert!(schema.contains("ManifestResponse"));
        assert!(schema.contains("WarmRequest"));
        assert!(schema.contains("WarmResponse"));
    }
}
