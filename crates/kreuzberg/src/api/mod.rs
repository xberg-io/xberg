//! REST API server for Kreuzberg document extraction.
//!
//! This module provides an Axum-based HTTP server for document extraction
//! with endpoints for single and batch extraction operations.
//!
//! # Endpoints
//!
//! - `POST /extract` - Extract text from uploaded files (multipart form data)
//! - `POST /embed` - Generate embeddings for text (JSON body with texts array)
//! - `POST /chunk` - Chunk text into smaller pieces (JSON body with text and config)
//! - `GET /health` - Health check endpoint
//! - `GET /info` - Server information
//! - `GET /cache/stats` - Get cache statistics
//! - `DELETE /cache/clear` - Clear all cached files
//!
//! # Examples
//!
//! ## Starting the server
//!
//! ```no_run
//! use kreuzberg::api::serve;
//!
//! #[tokio::main]
//! async fn main() -> kreuzberg::Result<()> {
//!     // Local development
//!     serve("127.0.0.1", 8000).await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Embedding the router in your app
//!
//! ```no_run
//! use kreuzberg::{ExtractionConfig, api::create_router};
//! use axum::Router;
//!
//! #[tokio::main]
//! async fn main() -> kreuzberg::Result<()> {
//!     // Load config (from file or use default)
//!     let config = ExtractionConfig::default();
//!     let kreuzberg_router = create_router(config);
//!
//!     // Nest under /api prefix
//!     let app = Router::new().nest("/api", kreuzberg_router);
//!
//!     // Add your own routes
//!     // ...
//!
//!     Ok(())
//! }
//! ```
//!
//! # cURL Examples
//!
//! ```bash
//! # Single file extraction
//! curl -F "files=@document.pdf" http://localhost:8000/extract
//!
//! # Multiple files with OCR config
//! curl -F "files=@doc1.pdf" -F "files=@doc2.jpg" \
//!      -F 'config={"ocr":{"language":"eng"}}' \
//!      http://localhost:8000/extract
//!
//! # Health check
//! curl http://localhost:8000/health
//!
//! # Server info
//! curl http://localhost:8000/info
//!
//! # Cache statistics
//! curl http://localhost:8000/cache/stats
//!
//! # Clear cache
//! curl -X DELETE http://localhost:8000/cache/clear
//!
//! # Generate embeddings
//! curl -X POST http://localhost:8000/embed \
//!      -H "Content-Type: application/json" \
//!      -d '{"texts":["Hello world","Second text"]}'
//!
//! # Chunk text
//! curl -X POST http://localhost:8000/chunk \
//!      -H "Content-Type: application/json" \
//!      -d '{"text":"Long text to chunk...","chunker_type":"text"}'
//! ```

mod error;
mod handlers;
mod server;
mod types;

pub use error::ApiError;
pub use server::{
    create_router, create_router_with_limits, create_router_with_limits_and_server_config, load_server_config, serve,
    serve_default, serve_with_config, serve_with_config_and_limits, serve_with_server_config,
};
pub use types::{
    ApiSizeLimits, ApiState, CacheClearResponse, CacheStatsResponse, ChunkRequest, ChunkResponse, EmbedRequest,
    EmbedResponse, ErrorResponse, ExtractResponse, HealthResponse, InfoResponse,
};
