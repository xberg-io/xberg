# Quality Processing

Score extracted text for quality issues (0.0–1.0, where 1.0 is highest quality). Detects OCR artifacts, script content, navigation elements, and structural issues to filter low-quality extractions before downstream processing.

| Factor              | Weight | Detects                                                |
| ------------------- | ------ | ------------------------------------------------------ |
| OCR Artifacts       | 30%    | Scattered chars, repeated punctuation, malformed words |
| Script Content      | 20%    | JavaScript, CSS, HTML tags                             |
| Navigation Elements | 10%    | Breadcrumbs, pagination, skip links                    |
| Document Structure  | 20%    | Sentence/paragraph length, punctuation distribution    |
| Metadata Quality    | 10%    | Presence of title, author, subject                     |

Score ranges: `0.0–0.3` very low, `0.3–0.6` low, `0.6–0.8` moderate, `0.8–1.0` high.

## Configuration

=== "Python"

    --8<-- "snippets/python/config/quality_processing_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/quality_processing_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/quality_processing_config.md"

=== "Go"

    --8<-- "snippets/go/config/quality_processing_config.md"

=== "Java"

    --8<-- "snippets/java/config/quality_processing_config.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/quality_processing_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/quality_processing_config.md"

## Example

=== "Python"

    --8<-- "snippets/python/utils/quality_processing_example.md"

=== "TypeScript"

    --8<-- "snippets/typescript/utils/quality_processing_example.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/quality_processing_example.md"

=== "Go"

    --8<-- "snippets/go/advanced/quality_processing_example.md"

=== "Java"

    --8<-- "snippets/java/advanced/quality_processing_example.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/quality_processing_example.md"

=== "Ruby"

    --8<-- "snippets/ruby/advanced/quality_processing_example.md"

## See also

- [Configuration Reference](../reference/configuration.md#ocrqualitythresholds) — all quality options
- [Extraction Basics](extraction.md) — core extraction pipeline
