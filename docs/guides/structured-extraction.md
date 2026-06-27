# Structured Extraction

Define a JSON schema and extract typed structured data directly from documents via LLM. Get back fields ready for your database or app, no manual parsing of raw text required.

Structured extraction is part of the unified extraction pipeline. Set
`ExtractionConfig.structured_extraction`, call `extract` or `extract_batch`, and
read `structured_output` from each `ExtractedDocument` in the returned
`ExtractionResult` envelope.

There is no separate public structured-extraction entrypoint in v1.

## Configure

Provide a JSON schema and an LLM model. Xberg first extracts the document, then
sends the extracted content to the configured model and stores the parsed JSON
result on the extracted document.

=== "Rust"
    ```rust
    use serde_json::json;
    use xberg::{
        extract, ExtractInput, ExtractionConfig, LlmConfig,
        StructuredExtractionConfig,
    };

    #[tokio::main]
    async fn main() -> xberg::Result<()> {
        let config = ExtractionConfig {
            structured_extraction: Some(StructuredExtractionConfig {
                schema_name: "paper_metadata".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "authors": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "date": { "type": "string" }
                    },
                    "required": ["title", "authors"],
                    "additionalProperties": false
                }),
                strict: true,
                llm: LlmConfig {
                    model: "openai/gpt-4o-mini".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            }),
            ..Default::default()
        };

        let output = extract(ExtractInput::from_uri("paper.pdf"), &config).await?;
        if let Some(result) = output.results.first() {
            if let Some(structured) = &result.structured_output {
                println!("{structured}");
            }
        }

        Ok(())
    }
    ```

=== "Python"
    ```python
    import asyncio
    from xberg import (
        ExtractInput,
        ExtractionConfig,
        LlmConfig,
        StructuredExtractionConfig,
        extract,
    )


    async def main() -> None:
        config = ExtractionConfig(
            structured_extraction=StructuredExtractionConfig(
                schema_name="paper_metadata",
                schema={
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"},
                        "authors": {
                            "type": "array",
                            "items": {"type": "string"},
                        },
                        "date": {"type": "string"},
                    },
                    "required": ["title", "authors"],
                    "additionalProperties": False,
                },
                strict=True,
                llm=LlmConfig(model="openai/gpt-4o-mini"),
            ),
        )

        output = await extract(ExtractInput.from_uri("paper.pdf"), config)
        if output.results and output.results[0].structured_output is not None:
            print(output.results[0].structured_output)


    asyncio.run(main())
    ```

=== "TypeScript"
    ```typescript
    import {
      ExtractInputKind,
      extract,
      type ExtractionConfig,
    } from "@xberg-io/xberg";

    const config: ExtractionConfig = {
      structuredExtraction: {
        schemaName: "paper_metadata",
        schema: {
          type: "object",
          properties: {
            title: { type: "string" },
            authors: {
              type: "array",
              items: { type: "string" },
            },
            date: { type: "string" },
          },
          required: ["title", "authors"],
          additionalProperties: false,
        },
        strict: true,
        llm: {
          model: "openai/gpt-4o-mini",
        },
      },
    };

    const output = await extract(
      { kind: ExtractInputKind.Uri, uri: "paper.pdf" },
      config,
    );

    const structured = output.results[0]?.structuredOutput;
    if (structured !== undefined && structured !== null) {
      console.log(structured);
    }
    ```

## TOML

The same configuration can be loaded from TOML:

```toml
[structured_extraction]
schema_name = "paper_metadata"
strict = true

[structured_extraction.schema]
type = "object"
required = ["title", "authors"]
additionalProperties = false

[structured_extraction.schema.properties.title]
type = "string"

[structured_extraction.schema.properties.authors]
type = "array"

[structured_extraction.schema.properties.authors.items]
type = "string"

[structured_extraction.schema.properties.date]
type = "string"

[structured_extraction.llm]
model = "openai/gpt-4o-mini"
```

## URLs And Batches

Structured extraction works with every `ExtractInput` source:

- `kind = "bytes"` for in-memory content
- `kind = "uri"` for local paths and `file://` URIs
- `kind = "uri"` for HTTP(S) document URLs and website crawl seeds

For batches, each successful result can carry its own `structured_output`.
Failures are reported in `ExtractionResult.errors` without discarding other
results.

## Output

Read structured data from each `ExtractedDocument.structured_output`.

The extraction envelope still includes normal document content, pages, chunks,
metadata, warnings, and errors. This lets downstream code store the raw
extraction and the structured projection together.

## Best Practices

Use strict schemas when the provider supports them. Keep schemas small and
specific, and include only fields you will consume. Set a model through
`LlmConfig`; credentials can come from the provider environment variable or
from `llm.api_key`.
