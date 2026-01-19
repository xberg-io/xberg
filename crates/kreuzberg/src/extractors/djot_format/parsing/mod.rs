//! Djot event parsing and content extraction.
//!
//! Handles parsing of jotdown events into plain text, tables, and full DjotContent structures.

mod block_handlers;
mod content_extraction;
mod event_handlers;
mod inline_handlers;
mod state;
mod table_extraction;
mod text_extraction;

// Re-export public API for backward compatibility
pub use content_extraction::extract_complete_djot_content;
pub use table_extraction::extract_tables_from_events;
pub use text_extraction::extract_text_from_events;
