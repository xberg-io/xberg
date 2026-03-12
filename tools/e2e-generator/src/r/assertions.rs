use super::render::{render_numeric_literal, render_r_string, render_r_value, render_string_vector};
use crate::fixtures::Assertions;

/// Generate R assertion code for a fixture's assertions
pub fn render_assertions(assertions: &Assertions) -> String {
    let mut buf = String::new();

    if !assertions.expected_mime.is_empty() {
        buf.push_str(&format!(
            "      assert_expected_mime(result, {})\n",
            render_string_vector(&assertions.expected_mime)
        ));
    }

    if let Some(min) = assertions.min_content_length {
        buf.push_str(&format!(
            "      assert_min_content_length(result, {})\n",
            render_numeric_literal(min as u64)
        ));
    }

    if let Some(max) = assertions.max_content_length {
        buf.push_str(&format!(
            "      assert_max_content_length(result, {})\n",
            render_numeric_literal(max as u64)
        ));
    }

    if !assertions.content_contains_any.is_empty() {
        buf.push_str(&format!(
            "      assert_content_contains_any(result, {})\n",
            render_string_vector(&assertions.content_contains_any)
        ));
    }

    if !assertions.content_contains_all.is_empty() {
        buf.push_str(&format!(
            "      assert_content_contains_all(result, {})\n",
            render_string_vector(&assertions.content_contains_all)
        ));
    }

    if let Some(tables) = assertions.tables.as_ref() {
        let min_lit = tables
            .min
            .map(|v| render_numeric_literal(v as u64))
            .unwrap_or_else(|| "NULL".into());
        let max_lit = tables
            .max
            .map(|v| render_numeric_literal(v as u64))
            .unwrap_or_else(|| "NULL".into());
        buf.push_str(&format!(
            "      assert_table_count(result, minimum = {}, maximum = {})\n",
            min_lit, max_lit
        ));
        if tables.has_bounding_boxes == Some(true) {
            buf.push_str("      assert_table_bounding_boxes(result)\n");
        }
        if let Some(snippets) = tables.content_contains_any.as_ref()
            && !snippets.is_empty()
        {
            buf.push_str(&format!(
                "      assert_table_content_contains_any(result, {})\n",
                render_string_vector(snippets)
            ));
        }
    }

    if let Some(languages) = assertions.detected_languages.as_ref() {
        let expected = render_string_vector(&languages.expects);
        let min_conf = languages
            .min_confidence
            .map(|v| v.to_string())
            .unwrap_or_else(|| "NULL".into());
        buf.push_str(&format!(
            "      assert_detected_languages(result, {}, {})\n",
            expected, min_conf
        ));
    }

    if !assertions.metadata.is_empty() {
        for (path, expectation) in &assertions.metadata {
            buf.push_str(&format!(
                "      assert_metadata_expectation(result, {}, {})\n",
                render_r_string(path),
                render_r_value(expectation)
            ));
        }
    }

    if let Some(chunks) = assertions.chunks.as_ref() {
        let mut args = Vec::new();
        if let Some(min) = chunks.min_count {
            args.push(format!("min_count = {}", render_numeric_literal(min as u64)));
        }
        if let Some(max) = chunks.max_count {
            args.push(format!("max_count = {}", render_numeric_literal(max as u64)));
        }
        if let Some(has_content) = chunks.each_has_content {
            args.push(format!(
                "each_has_content = {}",
                if has_content { "TRUE" } else { "FALSE" }
            ));
        }
        if let Some(has_embedding) = chunks.each_has_embedding {
            args.push(format!(
                "each_has_embedding = {}",
                if has_embedding { "TRUE" } else { "FALSE" }
            ));
        }
        if let Some(has_heading_context) = chunks.each_has_heading_context {
            args.push(format!(
                "each_has_heading_context = {}",
                if has_heading_context { "TRUE" } else { "FALSE" }
            ));
        }
        if !args.is_empty() {
            buf.push_str(&format!("      assert_chunks(result, {})\n", args.join(", ")));
        }
    }

    if let Some(images) = assertions.images.as_ref() {
        let mut args = Vec::new();
        if let Some(min) = images.min_count {
            args.push(format!("min_count = {}", render_numeric_literal(min as u64)));
        }
        if let Some(max) = images.max_count {
            args.push(format!("max_count = {}", render_numeric_literal(max as u64)));
        }
        if let Some(formats) = images.formats_include.as_ref() {
            args.push(format!("formats_include = {}", render_string_vector(formats)));
        }
        if !args.is_empty() {
            buf.push_str(&format!("      assert_images(result, {})\n", args.join(", ")));
        }
        if images.has_bounding_boxes == Some(true) {
            buf.push_str("      assert_image_bounding_boxes(result)\n");
        }
    }

    if let Some(pages) = assertions.pages.as_ref() {
        let mut args = Vec::new();
        if let Some(min) = pages.min_count {
            args.push(format!("min_count = {}", render_numeric_literal(min as u64)));
        }
        if let Some(exact) = pages.exact_count {
            args.push(format!("exact_count = {}", render_numeric_literal(exact as u64)));
        }
        if !args.is_empty() {
            buf.push_str(&format!("      assert_pages(result, {})\n", args.join(", ")));
        }
    }

    if let Some(elements) = assertions.elements.as_ref() {
        let mut args = Vec::new();
        if let Some(min) = elements.min_count {
            args.push(format!("min_count = {}", render_numeric_literal(min as u64)));
        }
        if let Some(types) = elements.types_include.as_ref() {
            args.push(format!("types_include = {}", render_string_vector(types)));
        }
        if !args.is_empty() {
            buf.push_str(&format!("      assert_elements(result, {})\n", args.join(", ")));
        }
    }

    if let Some(ocr) = assertions.ocr_elements.as_ref() {
        let mut args = Vec::new();
        if let Some(has_elements) = ocr.has_elements {
            args.push(format!(
                "has_elements = {}",
                if has_elements { "TRUE" } else { "FALSE" }
            ));
        }
        if let Some(has_geometry) = ocr.elements_have_geometry {
            args.push(format!(
                "elements_have_geometry = {}",
                if has_geometry { "TRUE" } else { "FALSE" }
            ));
        }
        if let Some(has_confidence) = ocr.elements_have_confidence {
            args.push(format!(
                "elements_have_confidence = {}",
                if has_confidence { "TRUE" } else { "FALSE" }
            ));
        }
        if let Some(min) = ocr.min_count {
            args.push(format!("min_count = {}", render_numeric_literal(min as u64)));
        }
        if !args.is_empty() {
            buf.push_str(&format!("      assert_ocr_elements(result, {})\n", args.join(", ")));
        }
    }

    if let Some(document) = assertions.document.as_ref() {
        let mut args = vec![format!(
            "has_document = {}",
            if document.has_document { "TRUE" } else { "FALSE" }
        )];
        if let Some(min_count) = document.min_node_count {
            args.push(format!("min_node_count = {}", render_numeric_literal(min_count as u64)));
        }
        if !document.node_types_include.is_empty() {
            args.push(format!(
                "node_types_include = {}",
                render_string_vector(&document.node_types_include)
            ));
        }
        if let Some(has_groups) = document.has_groups {
            args.push(format!("has_groups = {}", if has_groups { "TRUE" } else { "FALSE" }));
        }
        buf.push_str(&format!("      assert_document(result, {})\n", args.join(", ")));
    }

    if let Some(keywords) = assertions.keywords.as_ref() {
        let mut args = Vec::new();
        if let Some(has_keywords) = keywords.has_keywords {
            args.push(format!(
                "has_keywords = {}",
                if has_keywords { "TRUE" } else { "FALSE" }
            ));
        }
        if let Some(min) = keywords.min_count {
            args.push(format!("min_count = {}", render_numeric_literal(min as u64)));
        }
        if let Some(max) = keywords.max_count {
            args.push(format!("max_count = {}", render_numeric_literal(max as u64)));
        }
        if !args.is_empty() {
            buf.push_str(&format!("      assert_keywords(result, {})\n", args.join(", ")));
        }
    }

    if assertions.content_not_empty == Some(true) {
        buf.push_str("      assert_content_not_empty(result)\n");
    }

    if let Some(qs) = assertions.quality_score.as_ref() {
        let mut args = Vec::new();
        if let Some(has_score) = qs.has_score {
            args.push(format!("has_score = {}", if has_score { "TRUE" } else { "FALSE" }));
        }
        if let Some(min_score) = qs.min_score {
            args.push(format!("min_score = {}", min_score));
        }
        if let Some(max_score) = qs.max_score {
            args.push(format!("max_score = {}", max_score));
        }
        if !args.is_empty() {
            buf.push_str(&format!("      assert_quality_score(result, {})\n", args.join(", ")));
        }
    }

    if let Some(pw) = assertions.processing_warnings.as_ref() {
        let mut args = Vec::new();
        if let Some(max_count) = pw.max_count {
            args.push(format!("max_count = {}", render_numeric_literal(max_count as u64)));
        }
        if let Some(is_empty) = pw.is_empty {
            args.push(format!("is_empty = {}", if is_empty { "TRUE" } else { "FALSE" }));
        }
        if !args.is_empty() {
            buf.push_str(&format!(
                "      assert_processing_warnings(result, {})\n",
                args.join(", ")
            ));
        }
    }

    if let Some(dc) = assertions.djot_content.as_ref() {
        let mut args = Vec::new();
        if let Some(has_content) = dc.has_content {
            args.push(format!("has_content = {}", if has_content { "TRUE" } else { "FALSE" }));
        }
        if let Some(min_blocks) = dc.min_blocks {
            args.push(format!("min_blocks = {}", render_numeric_literal(min_blocks as u64)));
        }
        if !args.is_empty() {
            buf.push_str(&format!("      assert_djot_content(result, {})\n", args.join(", ")));
        }
    }

    if let Some(annotations) = assertions.annotations.as_ref() {
        let mut args = vec![format!(
            "has_annotations = {}",
            if annotations.has_annotations { "TRUE" } else { "FALSE" }
        )];
        if let Some(min_count) = annotations.min_count {
            args.push(format!("min_count = {}", render_numeric_literal(min_count as u64)));
        }
        buf.push_str(&format!("      assert_annotations(result, {})\n", args.join(", ")));
    }

    buf
}
