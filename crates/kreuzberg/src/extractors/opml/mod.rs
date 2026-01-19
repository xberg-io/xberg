//! Native OPML (Outline Processor Markup Language) extractor using the `roxmltree` library.
//!
//! This extractor provides native Rust-based OPML extraction, parsing outline structures
//! commonly used for RSS feed lists, podcast directories, and general outlines.
//!
//! Extracts:
//! - Metadata from `<head>`: title, dateCreated, dateModified, ownerName, ownerEmail
//! - Content from `<body><outline>` hierarchy using text attributes
//! - Outline hierarchy structure preserved in plain text format with indentation
//! - Note: URLs (xmlUrl, htmlUrl) are extracted from attributes but not included in main content
//!
//! Example OPML structure:
//! ```xml
//! <opml version="2.0">
//!   <head>
//!     <title>My Feeds</title>
//!     <ownerName>John</ownerName>
//!   </head>
//!   <body>
//!     <outline text="Tech" type="folder">
//!       <outline text="Hacker News" type="rss" xmlUrl="https://..." />
//!     </outline>
//!   </body>
//! </opml>
//! ```

mod core;
mod parser;

// Re-export public API
pub use core::OpmlExtractor;
