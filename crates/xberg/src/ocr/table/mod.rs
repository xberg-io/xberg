/// TSV (tab-separated value) output parser for Tesseract word-level bounding boxes.
pub mod tsv_parser;

mod coverage;
mod grid_shape;
mod shading_marks;

#[cfg(paddle_ocr)]
pub(crate) use crate::table_core::HocrWord;
pub(crate) use crate::table_core::{orient_table_for_reading_order, reconstruct_table_with_columns, table_to_markdown};

#[cfg(feature = "pdf")]
pub(crate) use crate::pdf::table_reconstruct::post_process_table;

pub(crate) use coverage::{
    drop_document_elements_claimed_by_tables, restore_words_of_rejected_regions, should_adopt_table_rebuild,
};
pub(crate) use grid_shape::has_table_grid;
pub(crate) use shading_marks::shading_mark_keep_mask;
pub(crate) use tsv_parser::{TableWords, extract_table_words_from_tsv, extract_words_from_tsv};
