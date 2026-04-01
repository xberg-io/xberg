# Keyword Extraction

Extract ranked keywords from document text using YAKE or RAKE algorithms.

| Algorithm | Scoring | Best for |
|-----------|---------|----------|
| **YAKE** | Lower score = more relevant (0.0–1.0) | General documents, single terms, multilingual |
| **RAKE** | Higher score = more relevant (unbounded) | Multi-word phrases, technical docs |

## Quick Start

=== "Python"

    --8<-- "snippets/python/utils/keyword_extraction_example.md"

=== "TypeScript"

    --8<-- "snippets/typescript/utils/keyword_extraction_example.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/keyword_extraction_example.md"

=== "Go"

    --8<-- "snippets/go/utils/keyword_extraction_example.md"

=== "Java"

    --8<-- "snippets/java/utils/keyword_extraction_example.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/keyword_extraction_example.md"

=== "Ruby"

    --8<-- "snippets/ruby/utils/keyword_extraction_example.md"

Keywords are returned in `result.extracted_keywords` as objects with `text` and `score` fields.

## Configuration

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `algorithm` | `KeywordAlgorithm` | `YAKE` | `YAKE` or `RAKE` |
| `max_keywords` | `int` | `10` | Maximum keywords to extract |
| `min_score` | `float` | `0.0` | Score threshold (upper bound for YAKE, lower bound for RAKE) |
| `ngram_range` | `tuple[int, int]` | `(1, 3)` | Min and max phrase length in words |
| `language` | `str \| None` | `"en"` | Language for stopword filtering (`None` disables) |
| `yake_params` | `YakeParams` | — | YAKE-specific tuning |
| `rake_params` | `RakeParams` | — | RAKE-specific tuning |

=== "Python"

    --8<-- "snippets/python/config/keyword_extraction_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/keyword_extraction_config.md"

=== "Rust"

    --8<-- "snippets/rust/config/keyword_extraction_config.md"

=== "Go"

    --8<-- "snippets/go/config/keyword_extraction_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/keyword_extraction_config.md"

=== "R"

    --8<-- "snippets/r/config/keyword_extraction_config.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/keyword_extraction_config.md"

## YAKE Score Tuning

Lower YAKE scores = higher relevance. Use `min_score` as an upper bound:

| `min_score` | Effect |
|-------------|--------|
| `0.5` | Keeps most keywords |
| `0.3` | Main topics only |
| `0.1` | Core concepts only |

`YakeParams.window_size` controls co-occurrence context: `1–2` for narrow domains, `2–3` for general (default: `2`), `3–4` for discussion-heavy content.

## RAKE Score Tuning

Higher RAKE scores = higher relevance. Use `min_score` as a lower bound:

| `min_score` | Effect |
|-------------|--------|
| `0.1` | Keeps most keywords |
| `5.0` | Main phrases only |
| `20.0` | Only highly specific phrases |

`RakeParams`: `min_word_length` (default: `1`), `max_words_per_phrase` (default: `3`).

## Troubleshooting

- **Too few keywords** — Lower `min_score`, check `result.content` is non-empty, set `language` to match the document or `None` to disable stopword filtering
- **Too many irrelevant keywords** — Raise `min_score`, set `language` for stopword filtering, reduce `ngram_range` upper bound
- **Multi-word phrases missing (YAKE)** — Switch to RAKE or confirm `ngram_range` upper bound is >= 2
- **Keywords don't match content** — Verify text was extracted (`result.content`) and `language` matches the document

See the [KeywordConfig reference](../reference/configuration.md#keywordconfig) for the full parameter list.
