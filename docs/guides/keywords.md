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

=== "Ruby"

    --8<-- "snippets/ruby/utils/keyword_extraction_example.md"

=== "Go"

    --8<-- "snippets/go/utils/keyword_extraction_example.md"

=== "Java"

    --8<-- "snippets/java/utils/keyword_extraction_example.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/keyword_extraction_example.md"

Keywords are returned in `result.extracted_keywords` as a list of objects with `text` and `score` fields.

## Configuration

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `algorithm` | `KeywordAlgorithm` | `YAKE` | Algorithm to use: `YAKE` or `RAKE` |
| `max_keywords` | `int` | `10` | Maximum keywords to extract |
| `min_score` | `float` | `0.0` | Score threshold. For YAKE: upper bound (lower scores kept). For RAKE: lower bound (higher scores kept). |
| `ngram_range` | `tuple[int, int]` | `(1, 3)` | Min and max phrase length in words |
| `language` | `str \| None` | `"en"` | Language code for stopword filtering. `None` disables filtering. |
| `yake_params` | `YakeParams` | — | YAKE-specific tuning |
| `rake_params` | `RakeParams` | — | RAKE-specific tuning |

=== "Python"

    --8<-- "snippets/python/config/keyword_extraction_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/keyword_extraction_config.md"

=== "Rust"

    --8<-- "snippets/rust/config/keyword_extraction_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/keyword_extraction_config.md"

=== "Go"

    --8<-- "snippets/go/config/keyword_extraction_config.md"

=== "R"

    --8<-- "snippets/r/config/keyword_extraction_config.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/keyword_extraction_config.md"

## YAKE

YAKE scores range from 0.0 to 1.0. Lower scores indicate higher relevance. Set `min_score` as an upper bound to keep only the most relevant terms.

| `min_score` | Effect |
|-------------|--------|
| `0.5` | Keeps most keywords |
| `0.3` | Main topics only |
| `0.1` | Core concepts only |

`YakeParams.window_size` controls the co-occurrence context window:

| `window_size` | Use when |
|---------------|----------|
| `1–2` | Narrow domains, technical documents |
| `2–3` | General-purpose (default: `2`) |
| `3–4` | News, discussion-heavy content |

## RAKE

RAKE scores are unbounded. Higher scores indicate higher relevance. Set `min_score` as a lower bound to filter noise.

| `min_score` | Effect |
|-------------|--------|
| `0.1` | Keeps most keywords |
| `5.0` | Main phrases only |
| `20.0` | Only highly specific phrases |

`RakeParams` options:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `min_word_length` | `1` | Ignore words shorter than this |
| `max_words_per_phrase` | `3` | Maximum words per extracted phrase |

## Troubleshooting

**Too few keywords returned:**

- `min_score` is too restrictive — lower the threshold
- Document has too little text — check `result.content` is non-empty
- Language mismatch — set `language` to match the document language, or `None` to disable stopword filtering

**Too many irrelevant keywords:**

- `min_score` too permissive — raise the threshold
- `language` not set — stopwords aren't filtered without a language code
- `ngram_range` upper bound too high — reduce it

**Multi-word phrases missing when using YAKE:**

- Switch to RAKE: it's purpose-built for phrase extraction
- Confirm `ngram_range` upper bound is `≥ 2`

**Keywords don't match document content:**

- Verify text was extracted: `result.content` should be non-empty
- Check that `language` matches the actual document language

See the [KeywordConfig reference](../reference/configuration.md#keywordconfig) for the full parameter list.
