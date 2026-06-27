# Layout Detection

Detect document layout regions (tables, figures, headers, text blocks, etc.) in PDFs using ONNX-based deep learning models. Enables table extraction, figure isolation, reading-order reconstruction, and selective OCR.

!!! Note "Feature gate" Requires the `layout-detection` Cargo feature. Not included in the default feature set.

## Model

Layout detection uses the **RT-DETR v2** model, an ONNX-based deep learning model that detects 17 layout element classes: text blocks, tables, figures, headers, footers, captions, code, lists, sections, formulas, footnotes, page headers/footers, titles, checkboxes, key-value regions, and document indices.

Layout detection now populates `ExtractionResult.formulas` for formula regions and supports chart understanding via `enable_chart_understanding`.

### When to Enable

**Recommended for:** complex multi-column PDFs, scanned documents, academic papers, business forms, and any document where layout understanding improves extraction accuracy.

**Less beneficial for:** simple single-column text documents, high-throughput pipelines where latency is critical (consider GPU acceleration), or documents already well-handled by PDF structure trees.

### Performance Impact

| Pipeline | Structure F1 | Text F1 | Avg time/doc |
| -------- | ------------ | ------- | ------------ |
| Baseline | 33.9%        | 87.4%   | 447 ms       |
| Layout   | 41.1%        | 90.1%   | 1500 ms      |

_171-document PDF corpus, CPU only. GPU acceleration significantly reduces the time penalty._

!!! Note "Layout Detection Model" Xberg uses only the RT-DETR v2 model for layout detection. The `preset` field is not available in `LayoutDetectionConfig`. Configure table structure recognition separately via `table_model` — see "Table Structure Models" below.

## Configuration

=== "Python"

    ```python
    from xberg import ExtractionConfig, LayoutDetectionConfig, extract

    config = ExtractionConfig(
        layout=LayoutDetectionConfig(
            confidence_threshold=0.5,
            apply_heuristics=True,
            table_model="tatr",
        )
    )
    result = await extract("document.pdf", config=config)
    ```

=== "TypeScript"

    ```typescript
    const result = await extract("document.pdf", {
      layout: {
        confidenceThreshold: 0.5,
        applyHeuristics: true,
        tableModel: "tatr",
      },
    });
    ```

=== "Rust"

    ```rust
    use xberg::core::{ExtractionConfig, LayoutDetectionConfig};

    let config = ExtractionConfig {
        layout: Some(LayoutDetectionConfig {
            confidence_threshold: Some(0.5),
            apply_heuristics: true,
            table_model: Some("tatr".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

=== "TOML"

    ```toml title="xberg.toml"
    [layout]
    apply_heuristics = true
    # table_model = "tatr"
    ```

=== "CLI"

    ```bash title="Terminal"
    # Enable layout detection with default settings
    xberg extract document.pdf --layout --content-format markdown

    # Custom confidence threshold
    xberg extract document.pdf --layout-confidence 0.5 --content-format markdown

    # Specific table model
    xberg extract document.pdf --layout --layout-table-model slanet_wired

    # Combined with GPU acceleration
    xberg extract document.pdf --layout --acceleration coreml
    ```

See [LayoutDetectionConfig](../reference/configuration.md#layoutdetectionconfig) for all fields.

## Table Structure Models

When layout detection identifies a table region, a table structure model analyzes rows, columns, headers, and spanning cells. Set `LayoutDetectionConfig.table_model` to one of:

| Value             | Notes                                                       |
| ----------------- | ----------------------------------------------------------- |
| `tatr`            | Default. Fast (~30 MB). General-purpose.                    |
| `slanet_wired`    | Higher accuracy for bordered/gridlined tables (~365 MB).    |
| `slanet_wireless` | Higher accuracy for borderless tables (~365 MB).            |
| `slanet_auto`     | Auto-classifies per page (~737 MB). Slowest.                |
| `slanet_plus`     | Smallest (~7.78 MB). For resource-constrained environments. |
| `disabled`        | Skip table structure recognition.                           |

!!! Note "Model Download" SLANeXT models are not downloaded by default. Use `cache warm --all-table-models` to pre-download, or they download automatically on first use.

## GPU Acceleration

Layout detection uses ONNX Runtime with automatic provider selection:

| Provider | Platform       | Notes                         |
| -------- | -------------- | ----------------------------- |
| CPU      | All            | Default, no setup needed      |
| CUDA     | Linux, Windows | Requires CUDA toolkit + cuDNN |
| CoreML   | macOS          | Automatic on Apple Silicon    |
| TensorRT | Linux          | Requires TensorRT             |

To override:

```python
config = ExtractionConfig(
    layout=LayoutDetectionConfig(),
    acceleration=AccelerationConfig(provider="cuda", device_id=0)
)
```

See [AccelerationConfig reference](../reference/configuration.md#accelerationconfig) for details.

## Layout Classes

The RT-DETR v2 model detects 17 classes. Each `LayoutRegion.class_name` is one of:

`caption`, `footnote`, `formula`, `list_item`, `page_footer`, `page_header`, `picture`, `section_header`, `table`, `text`, `title`, `document_index`, `code`, `checkbox_selected`, `checkbox_unselected`, `form`, `key_value_region`.

See [`LayoutRegion`](../reference/types.md) in the types reference for the full field shape.

## Accessing Layout Regions

When layout detection is enabled AND page extraction is enabled, each page in the result includes `layout_regions` — a list of detected regions with class, confidence score, bounding box, and area fraction. This enables programmatic filtering and analysis of specific layout elements.

=== "Python"

    ```python
    from xberg import extract, ExtractionConfig, LayoutDetectionConfig, PagesConfig

    result = await extract(
        "document.pdf",
        config=ExtractionConfig(
            layout=LayoutDetectionConfig(),
            pages=PagesConfig(extract_pages=True),
        ),
    )

    for page in result.pages:
        if page.layout_regions:
            for region in page.layout_regions:
                if region.class_name == "picture" and region.confidence > 0.9:
                    print(f"Page {page.page_number}: diagram detected "
                          f"(confidence={region.confidence:.2f}, "
                          f"area={region.area_fraction:.0%})")
    ```

=== "TypeScript"

    ```typescript
    const result = await extract("document.pdf", {
      layout: {},
      pages: { extractPages: true },
    });

    for (const page of result.pages ?? []) {
      if (page.layoutRegions) {
        for (const region of page.layoutRegions) {
          if (region.className === "picture" && region.confidence > 0.9) {
            console.log(
              `Page ${page.pageNumber}: diagram detected ` +
              `(confidence=${region.confidence.toFixed(2)}, ` +
              `area=${(region.areaFraction * 100).toFixed(0)}%)`
            );
          }
        }
      }
    }
    ```

=== "Rust"

    ```rust
    use xberg::core::{ExtractionConfig, LayoutDetectionConfig, PagesConfig};

    let result = extract(
        "document.pdf",
        ExtractionConfig {
            layout: Some(LayoutDetectionConfig::default()),
            pages: Some(PagesConfig {
                extract_pages: true,
                ..Default::default()
            }),
            ..Default::default()
        },
    ).await?;

    for page in &result.pages {
        if let Some(regions) = &page.layout_regions {
            for region in regions {
                if region.class_name == "picture" && region.confidence > 0.9 {
                    println!(
                        "Page {}: diagram detected (confidence={:.2}, area={:.0}%)",
                        page.page_number,
                        region.confidence,
                        region.area_fraction * 100.0
                    );
                }
            }
        }
    }
    ```

### Tips

- Use `confidence` to filter low-confidence detections — typically ≥ 0.8–0.9 for downstream operations
- Use `area_fraction` to distinguish between inline images and full-page diagrams (e.g., `area_fraction > 0.1` for significant figures)
- Regions are independent of page extraction — enable both to access both content and layout structure
- Available across all bindings (Python, TypeScript, Rust, Ruby, Java, Go, Elixir, C#, PHP)

## Acknowledgments

- **[Docling](https://github.com/DS4SD/docling)** — RT-DETR v2 model and layout classification approach
- **[TATR](https://github.com/microsoft/table-transformer)** — Table structure recognition with ONNX
- **[PaddleOCR](https://github.com/PaddlePaddle/PaddleOCR)** — SLANeXT table structure and PP-LCNet classifier models

## Related

- [Configuration Reference](../reference/configuration.md#layoutdetectionconfig) — full field reference
- [Element-Based Output](output-formats.md#element-based-output) — using layout-aware results
