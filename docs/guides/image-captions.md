# VLM Image Captions <span class="version-badge">v5.0.0-rc.3</span>

Caption every extracted image with a vision-language model. Captions populate `ExtractedImage.caption` for every image whose area exceeds `CaptioningConfig.min_image_area`.

!!! Note "Feature gate"
    Requires the `captioning` Cargo feature. Included in `full`. Requires `liter-llm` and a vision-capable provider.

## When to Use

- You need alt-text for accessibility-compliant exports
- You need searchable text descriptions per image to feed into a retrieval pipeline alongside the document body
- You need diagrams, charts, or photos described for LLM downstream consumption

## When Not to Use

- You only need OCR'd text from images — use [OCR](ocr.md) for text extraction from images
- You're processing high-volume batches where API spend is a concern — captioning calls an LLM per image
- Images are mostly decorative or structural elements

## Configuration

=== "Python"

    --8<-- "snippets/python/captioning/basic.md"

=== "TypeScript"

    --8<-- "snippets/typescript/captioning/basic.md"

=== "Rust"

    --8<-- "snippets/rust/captioning/basic.md"

=== "TOML"

    --8<-- "snippets/cli/captioning_toml.md"

## Custom Prompt

Override the built-in caption prompt:

=== "Python"

    --8<-- "snippets/python/captioning/custom_prompt.md"

The prompt is sent alongside each image as a single VLM request. The model sees the image plus the prompt; the response becomes the caption verbatim.

## Filtering Small Images

`min_image_area` is in pixels (width × height). Icons, bullets, and decorative glyphs below the threshold are skipped — their `caption` field stays `None`. The default `1000` excludes 32×32 icons but admits typical inline figures. Raise the threshold to skip thumbnails; lower it to caption everything.

## Output Shape

```json
{
  "images": [
    {
      "image_kind": "diagram",
      "caption": "A flowchart showing the data ingestion pipeline: source → cleaner → indexer → retrieval API.",
      "bounding_box": { "page": 3, "x": 72, "y": 144, "width": 468, "height": 312 }
    },
    {
      "image_kind": "icon",
      "caption": null
    }
  ]
}
```

## Supported Providers

Any vision-capable liter-llm provider works (see the [VLM OCR provider table](llm-integration.md#supported-providers)). For batch captioning, `gpt-4o-mini`, `claude-3-5-haiku`, and `google/gemini-2.0-flash` are typically the cheapest options.

API-key precedence chain matches [LLM Integration](llm-integration.md#api-key-configuration):

1. `CaptioningConfig.llm.api_key`
2. `KREUZBERG_LLM_API_KEY`
3. Per-provider env var

Local engines (Ollama, LM Studio with a VLM, vLLM) need no key.

## Related

- [LLM Integration](llm-integration.md) — provider matrix, local engines, VLM OCR
- [OCR](ocr.md) — text-from-image extraction
- [Configuration Reference](../reference/configuration.md#captioningconfig)
