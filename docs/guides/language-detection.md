# Language Detection

Detect languages in extracted text using [`whatlang`](https://crates.io/crates/whatlang) — supports 60+ languages with ISO 639-3 codes. Set `detect_multiple: true` to chunk the text into 200-character segments and return all detected languages sorted by prevalence.

## Configuration

=== "Python"

    --8<-- "snippets/python/config/language_detection_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/language_detection_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/language_detection_config.md"

=== "Go"

    --8<-- "snippets/go/config/language_detection_config.md"

=== "Java"

    --8<-- "snippets/java/config/language_detection_config.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/language_detection_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/language_detection_config.md"

=== "R"

    --8<-- "snippets/r/config/language_detection_config.md"

## Multilingual Example

=== "Python"

    --8<-- "snippets/python/utils/language_detection_multilingual.md"

=== "TypeScript"

    --8<-- "snippets/typescript/metadata/language_detection_multilingual.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/language_detection_multilingual.md"

=== "Go"

    --8<-- "snippets/go/advanced/language_detection_multilingual.md"

=== "Java"

    --8<-- "snippets/java/advanced/language_detection_multilingual.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/language_detection_multilingual.md"

=== "Ruby"

    --8<-- "snippets/ruby/advanced/language_detection_multilingual.md"

=== "R"

    --8<-- "snippets/r/advanced/language_detection_multilingual.md"

## See also

- [Configuration Reference](../reference/configuration.md#languagedetectionconfig) — all detection options
- [Chunking](chunking.md) — split text before language detection for per-section analysis
