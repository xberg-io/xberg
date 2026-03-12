//! Chunk construction and building logic.
//!
//! This module handles the construction of individual chunks from text segments,
//! including overlap calculation, offset tracking, and metadata assembly.

use crate::error::{KreuzbergError, Result};
use crate::types::{Chunk, ChunkMetadata, PageBoundary};
use text_splitter::{Characters, ChunkCapacity, ChunkConfig};

use super::boundaries::calculate_page_range;

/// Build a ChunkConfig from chunking parameters.
///
/// # Arguments
///
/// * `max_characters` - Maximum characters per chunk
/// * `overlap` - Character overlap between consecutive chunks
/// * `trim` - Whether to trim whitespace from boundaries
///
/// # Returns
///
/// A configured ChunkConfig ready for use with text splitters.
///
/// # Errors
///
/// Returns `KreuzbergError::Validation` if configuration is invalid.
pub fn build_chunk_config(max_characters: usize, overlap: usize, trim: bool) -> Result<ChunkConfig<Characters>> {
    ChunkConfig::new(ChunkCapacity::new(max_characters))
        .with_overlap(overlap)
        .map(|config| config.with_trim(trim))
        .map_err(|e| KreuzbergError::validation(format!("Invalid chunking configuration: {}", e)))
}

/// Build chunks from text segments with optional page boundary tracking.
///
/// This function takes a collection of text segments (produced by a text splitter)
/// and constructs Chunk objects with proper metadata, including:
/// - Byte offsets derived from the chunk's position in the source text
/// - Chunk indices and total count
/// - Page boundary information (if provided)
///
/// # Arguments
///
/// * `source_text` - The original text that the chunks were split from. Chunk
///   slices must borrow from this text (as `text-splitter` guarantees).
/// * `text_chunks` - Iterator of text segments to convert into chunks
/// * `page_boundaries` - Optional page boundary markers for mapping chunks to pages
///
/// # Returns
///
/// A vector of Chunk objects with complete metadata.
///
/// # Errors
///
/// Returns an error if page boundary calculation fails.
pub fn build_chunks<'a, I>(
    source_text: &'a str,
    text_chunks: I,
    page_boundaries: Option<&[PageBoundary]>,
) -> Result<Vec<Chunk>>
where
    I: IntoIterator<Item = &'a str>,
{
    let chunks_vec: Vec<&str> = text_chunks.into_iter().collect();
    let total_chunks = chunks_vec.len();
    let source_start = source_text.as_ptr() as usize;
    let mut chunks = Vec::with_capacity(total_chunks);

    for (index, chunk_text) in chunks_vec.into_iter().enumerate() {
        let byte_start = chunk_text.as_ptr() as usize - source_start;
        let byte_end = byte_start + chunk_text.len();

        let (first_page, last_page) = if let Some(boundaries) = page_boundaries {
            calculate_page_range(byte_start, byte_end, boundaries)?
        } else {
            (None, None)
        };

        chunks.push(Chunk {
            content: chunk_text.to_string(),
            embedding: None,
            metadata: ChunkMetadata {
                byte_start,
                byte_end,
                token_count: None,
                chunk_index: index,
                total_chunks,
                first_page,
                last_page,
                heading_context: None,
            },
        });
    }

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_chunk_config_valid() {
        let result = build_chunk_config(100, 10, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_chunk_config_invalid_overlap() {
        let result = build_chunk_config(10, 20, true);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, KreuzbergError::Validation { .. }));
    }

    #[test]
    fn test_build_chunks_empty() {
        let source = "";
        let text_chunks: Vec<&str> = vec![];
        let result = build_chunks(source, text_chunks, None).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_build_chunks_single() {
        let source = "Single chunk";
        let result = build_chunks(source, vec![source], None).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "Single chunk");
        assert_eq!(result[0].metadata.chunk_index, 0);
        assert_eq!(result[0].metadata.total_chunks, 1);
        assert_eq!(result[0].metadata.byte_start, 0);
        assert_eq!(result[0].metadata.byte_end, 12);
    }

    #[test]
    fn test_build_chunks_with_page_boundaries() {
        let source = "First chunkSecond chunk";
        let text_chunks = vec![&source[0..11], &source[11..23]];
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 11,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 11,
                byte_end: 23,
                page_number: 2,
            },
        ];

        let result = build_chunks(source, text_chunks, Some(&boundaries)).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].metadata.first_page, Some(1));
        assert_eq!(result[1].metadata.first_page, Some(2));
    }

    #[test]
    fn test_build_chunks_offset_from_source() {
        let source = "AAAAABBBBBCCCCC";
        // Overlapping slices from source
        let text_chunks = vec![&source[0..5], &source[3..8], &source[6..11]];
        let result = build_chunks(source, text_chunks, None).unwrap();

        assert_eq!(result.len(), 3);

        assert_eq!(result[0].metadata.byte_start, 0);
        assert_eq!(result[0].metadata.byte_end, 5);

        assert_eq!(result[1].metadata.byte_start, 3);
        assert_eq!(result[1].metadata.byte_end, 8);

        assert_eq!(result[2].metadata.byte_start, 6);
        assert_eq!(result[2].metadata.byte_end, 11);
    }

    #[test]
    fn test_build_chunks_no_overlap() {
        let source = "AAAAABBBBBCCCCC";
        let text_chunks = vec![&source[0..5], &source[5..10], &source[10..15]];
        let result = build_chunks(source, text_chunks, None).unwrap();

        assert_eq!(result.len(), 3);

        assert_eq!(result[0].metadata.byte_start, 0);
        assert_eq!(result[0].metadata.byte_end, 5);

        assert_eq!(result[1].metadata.byte_start, 5);
        assert_eq!(result[1].metadata.byte_end, 10);

        assert_eq!(result[2].metadata.byte_start, 10);
        assert_eq!(result[2].metadata.byte_end, 15);
    }
}
