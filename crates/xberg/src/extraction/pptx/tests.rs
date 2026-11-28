use super::*;

mod chart_numbers;
mod namespace_prefixes;
mod ooxml_regressions;
mod run_spacing_and_reading_order;
mod slides_images_and_comments;

pub(crate) use ooxml_regressions::build_single_slide_pptx;
pub(crate) use slides_images_and_comments::{create_pptx_with_sections_and_notes, create_test_pptx_bytes};
