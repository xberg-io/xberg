//! MCP prompt definitions for guided document extraction workflows.
//!
//! Three prompts are provided:
//!
//! - `extract_document` — generate a Kreuzberg extraction call for a document path
//! - `extract_with_ocr` — extraction with specific OCR language configuration
//! - `semantic_search` — semantic search with embeddings and chunking

use rmcp::{
    handler::server::router::prompt::{PromptRoute, PromptRouter},
    model::{GetPromptResult, Prompt, PromptArgument, PromptMessage, PromptMessageRole},
};

/// Build and return a configured [`PromptRouter`] containing all three guided-workflow prompts.
pub fn build_prompt_router<S>() -> PromptRouter<S>
where
    S: Send + Sync + 'static,
{
    let mut router = PromptRouter::new();

    // --- extract_document ---
    router.add_route(PromptRoute::new_dyn(
        Prompt::new(
            "extract_document",
            Some("Generate a Kreuzberg extraction call for a document"),
            Some(vec![
                PromptArgument::new("path")
                    .with_description("Path to the document file")
                    .with_required(true),
                PromptArgument::new("output_format")
                    .with_description("Output format: 'json' (default) or 'toon'")
                    .with_required(false),
            ]),
        ),
        |context| {
            Box::pin(async move {
                let path = context
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("<path>")
                    .to_string();
                let fmt = context
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("output_format"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("json")
                    .to_string();
                Ok(GetPromptResult::new(vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "Extract text and metadata from the document at: {path}\n\
                         Output format: {fmt}\n\
                         Use the extract_file tool with path=\"{path}\" and \
                         response_format=\"{fmt}\"."
                    ),
                )]))
            })
        },
    ));

    // --- extract_with_ocr ---
    router.add_route(PromptRoute::new_dyn(
        Prompt::new(
            "extract_with_ocr",
            Some("Extract a document with explicit OCR language configuration"),
            Some(vec![
                PromptArgument::new("path")
                    .with_description("Path to the document or image file")
                    .with_required(true),
                PromptArgument::new("languages")
                    .with_description("Comma-separated ISO 639 language codes for OCR (e.g. 'eng,deu')")
                    .with_required(false),
                PromptArgument::new("force_ocr")
                    .with_description("Set to 'true' to force OCR even when native text is available")
                    .with_required(false),
            ]),
        ),
        |context| {
            Box::pin(async move {
                let path = context
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("<path>")
                    .to_string();
                let languages = context
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("languages"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("eng")
                    .to_string();
                let force_ocr = context
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("force_ocr"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);
                let lang_list: Vec<String> = languages.split(',').map(|s| format!("\"{s}\"")).collect();
                let force_ocr_str = if force_ocr { "true" } else { "false" };
                Ok(GetPromptResult::new(vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "Extract content from {path} using OCR.\n\
                         Use the extract_file tool with:\n\
                         - path=\"{path}\"\n\
                         - config={{\"enable_ocr\":true,\"force_ocr\":{force_ocr_str},\
                           \"ocr\":{{\"language\":[{langs}]}}}}\n\
                         OCR languages: {languages}",
                        langs = lang_list.join(","),
                    ),
                )]))
            })
        },
    ));

    // --- semantic_search ---
    router.add_route(PromptRoute::new_dyn(
        Prompt::new(
            "semantic_search",
            Some("Prepare a document for semantic search using embeddings and chunking"),
            Some(vec![
                PromptArgument::new("path")
                    .with_description("Path to the document to index")
                    .with_required(true),
                PromptArgument::new("preset")
                    .with_description("Embedding preset: 'speed', 'balanced' (default), or 'quality'")
                    .with_required(false),
                PromptArgument::new("chunker_type")
                    .with_description("Chunker strategy: 'text' (default), 'markdown', 'yaml', or 'semantic'")
                    .with_required(false),
                PromptArgument::new("max_characters")
                    .with_description("Maximum characters per chunk (default: 2000)")
                    .with_required(false),
            ]),
        ),
        |context| {
            Box::pin(async move {
                let path = context
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("<path>")
                    .to_string();
                let preset = context
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("preset"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("balanced")
                    .to_string();
                let chunker_type = context
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("chunker_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("text")
                    .to_string();
                let max_characters = context
                    .arguments
                    .as_ref()
                    .and_then(|a| a.get("max_characters"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("2000")
                    .to_string();
                Ok(GetPromptResult::new(vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    format!(
                        "Index {path} for semantic search:\n\
                         1. Extract text: call extract_file with path=\"{path}\"\n\
                         2. Chunk text: call chunk_text with \
                            chunker_type=\"{chunker_type}\" and max_characters={max_characters}\n\
                         3. Embed chunks: call embed_text with preset=\"{preset}\" \
                            on each chunk's content\n\
                         Store (chunk_text, embedding) pairs in your vector store."
                    ),
                )]))
            })
        },
    ));

    router
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_router_has_expected_prompts() {
        // Use () as a stand-in server type since the router is generic over S.
        let router = build_prompt_router::<()>();
        assert!(
            router.has_route("extract_document"),
            "extract_document should be registered"
        );
        assert!(
            router.has_route("extract_with_ocr"),
            "extract_with_ocr should be registered"
        );
        assert!(
            router.has_route("semantic_search"),
            "semantic_search should be registered"
        );
    }

    #[test]
    fn test_list_all_returns_three_prompts() {
        let router = build_prompt_router::<()>();
        let prompts = router.list_all();
        assert_eq!(prompts.len(), 3, "Expected exactly 3 prompts");
    }

    #[test]
    fn test_prompts_have_descriptions() {
        let router = build_prompt_router::<()>();
        for prompt in router.list_all() {
            assert!(
                prompt.description.is_some(),
                "Prompt '{}' should have a description",
                prompt.name
            );
        }
    }

    #[test]
    fn test_extract_document_has_required_path_argument() {
        let router = build_prompt_router::<()>();
        let prompt = router
            .list_all()
            .into_iter()
            .find(|p| p.name == "extract_document")
            .unwrap();
        let args = prompt.arguments.expect("extract_document should have arguments");
        let path_arg = args.iter().find(|a| a.name == "path").expect("path argument missing");
        assert_eq!(path_arg.required, Some(true), "path argument should be required");
    }

    #[test]
    fn test_extract_with_ocr_has_required_path_argument() {
        let router = build_prompt_router::<()>();
        let prompt = router
            .list_all()
            .into_iter()
            .find(|p| p.name == "extract_with_ocr")
            .unwrap();
        let args = prompt.arguments.expect("extract_with_ocr should have arguments");
        let path_arg = args.iter().find(|a| a.name == "path").expect("path argument missing");
        assert_eq!(path_arg.required, Some(true));
    }

    #[test]
    fn test_semantic_search_has_required_path_argument() {
        let router = build_prompt_router::<()>();
        let prompt = router
            .list_all()
            .into_iter()
            .find(|p| p.name == "semantic_search")
            .unwrap();
        let args = prompt.arguments.expect("semantic_search should have arguments");
        let path_arg = args.iter().find(|a| a.name == "path").expect("path argument missing");
        assert_eq!(path_arg.required, Some(true));
    }

    #[test]
    fn test_prompts_list_is_sorted() {
        let router = build_prompt_router::<()>();
        let prompts = router.list_all();
        let names: Vec<&str> = prompts.iter().map(|p| p.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "list_all should return prompts in sorted order");
    }
}
