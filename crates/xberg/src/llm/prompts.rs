//! Default Jinja2 prompt templates for LLM-based operations.
//!
//! Templates use [minijinja](https://docs.rs/minijinja) syntax. Users can
//! supply custom templates via the `prompt` configuration field using the
//! same variable names.
//!
//! # Available variables
//!
//! ## Structured extraction (`structured_extraction.prompt`)
//! - `{{ content }}` — The extracted document text.
//! - `{{ schema }}` — The JSON schema as a formatted string.
//! - `{{ schema_name }}` — The schema name.
//! - `{{ schema_description }}` — The schema description (may be empty).
//!
//! ## VLM OCR (`ocr.vlm_prompt`)
//! - `{{ language }}` — The document language code (e.g., "eng", "deu").

/// Default Jinja2 template for structured data extraction.
pub const STRUCTURED_EXTRACTION_TEMPLATE: &str = "\
You are a document data extraction system. Extract structured data from the \
following document content according to the provided JSON schema.

{% if schema_description %}Schema description: {{ schema_description }}
{% endif %}
Document content:
{{ content }}

Extract the requested data and return valid JSON that conforms to the schema. \
Return ONLY the JSON object, with no additional text, explanation, or markdown formatting.";

/// Default Jinja2 template for VLM OCR.
pub const VLM_OCR_TEMPLATE: &str = "\
Extract all visible text from this image. \
Reproduce the text exactly as it appears, preserving the original structure, \
paragraph breaks, and reading order. \
Transcribe every occurrence of repeated text in order: never merge, skip or \
summarize a paragraph, sentence or line because it repeats earlier text. \
Render mathematical formulas as LaTeX, wrapped in $$ for display equations \
and $ for inline ones. \
Do not add any commentary, explanation, or formatting beyond what is present \
in the image. \
If the image contains no text, respond with an empty string.\
{% if language and language != 'eng' and language != 'en' %}

The document is in language: {{ language }}\
{% endif %}";

/// Render a Jinja2 template with the given context variables.
pub(crate) fn render_template(template: &str, context: &minijinja::value::Value) -> crate::Result<String> {
    let env = minijinja::Environment::new();
    let tmpl = env
        .template_from_str(template)
        .map_err(|e| crate::XbergError::validation(format!("Invalid prompt template: {e}")))?;
    tmpl.render(context)
        .map_err(|e| crate::XbergError::validation(format!("Failed to render prompt template: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression test: VLM_OCR_TEMPLATE must instruct the model to render math as
    /// LaTeX, and the pre-existing `{% if language %}` block must still render
    /// correctly for both a non-English language and the English no-op case.
    #[test]
    fn test_vlm_ocr_template_includes_math_instruction_and_language_block() {
        let ctx_non_english = minijinja::context! { language => "deu" };
        let prompt = render_template(VLM_OCR_TEMPLATE, &ctx_non_english).unwrap();
        assert!(prompt.contains("LaTeX"), "must instruct math as LaTeX; got: {prompt}");
        assert!(
            prompt.contains("$$"),
            "must mention the display-math delimiter; got: {prompt}"
        );
        assert!(
            prompt.contains("language: deu"),
            "language block must still render; got: {prompt}"
        );

        let ctx_english = minijinja::context! { language => "eng" };
        let prompt_en = render_template(VLM_OCR_TEMPLATE, &ctx_english).unwrap();
        assert!(
            prompt_en.contains("LaTeX"),
            "math instruction must be unconditional; got: {prompt_en}"
        );
        assert!(
            !prompt_en.contains("language:"),
            "English must skip the language hint; got: {prompt_en}"
        );
    }

    #[test]
    fn test_vlm_ocr_template_requires_repeated_text_preserved_issue_2132() {
        let prompt = render_template(VLM_OCR_TEMPLATE, &minijinja::context! { language => "eng" }).unwrap();
        assert!(
            prompt.contains("Transcribe every occurrence of repeated text in order"),
            "must instruct preserving repeats; got: {prompt}"
        );
        assert!(
            prompt.contains("never merge, skip or summarize"),
            "must forbid collapsing repeats; got: {prompt}"
        );
    }
}
