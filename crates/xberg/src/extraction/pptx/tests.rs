use super::*;

mod chart_numbers;
mod ooxml_regressions;
mod slides_images_and_comments;

pub(crate) use ooxml_regressions::build_single_slide_pptx;
pub(crate) use slides_images_and_comments::{create_pptx_with_sections_and_notes, create_test_pptx_bytes};
