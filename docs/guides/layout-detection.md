# Layout Detection <span class="version-badge">v4.5.0</span>

Detect document layout regions (tables, figures, headers, text blocks, etc.) in PDFs using ONNX-based deep learning models. Enables table extraction, figure isolation, reading-order reconstruction, and selective OCR.

!!! note "Feature gate"
    Requires the `layout-detection` Cargo feature. Not included in the default feature set.

## Model Presets

| Preset | Model | Classes | Speed | Best for |
|--------|-------|---------|-------|----------|
| `"fast"` | YOLO DocLayNet | 11 | Fastest | High-throughput pipelines, general documents |
| `"accurate"` | RT-DETR v2 | 17 | Fast | Complex layouts, forms, mixed-content pages |

### When to Enable

**Recommended for:** complex multi-column PDFs, scanned documents, academic papers, business forms, documents where table extraction quality matters.

**Less beneficial for:** simple single-column text, high-throughput pipelines where 3.4x latency is unacceptable (consider GPU), documents already well-handled by the PDF structure tree.

### Performance Impact

| Pipeline | Structure F1 | Text F1 | Avg time/doc |
|----------|-------------|---------|--------------|
| Baseline | 33.9% | 87.4% | 447 ms |
| Layout | 41.1% | 90.1% | 1500 ms |

*171-document PDF corpus, CPU only. GPU acceleration significantly reduces the time penalty.*

## Configuration

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, LayoutDetectionConfig, extract_file

    config = ExtractionConfig(
        layout=LayoutDetectionConfig(
            preset="accurate",
            confidence_threshold=0.5,
            apply_heuristics=True,
            table_model="tatr",
        )
    )
    result = await extract_file("document.pdf", config=config)
    ```

=== "TypeScript"

    ```typescript
    const result = await extract("document.pdf", {
      layout: {
        preset: "accurate",
        confidenceThreshold: 0.5,
        applyHeuristics: true,
        tableModel: "tatr",
      },
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::core::{ExtractionConfig, LayoutDetectionConfig};

    let config = ExtractionConfig {
        layout: Some(LayoutDetectionConfig {
            preset: "accurate".to_string(),
            confidence_threshold: Some(0.5),
            apply_heuristics: true,
            table_model: Some("tatr".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

=== "TOML"

    ```toml title="kreuzberg.toml"
    [layout]
    preset = "fast"
    apply_heuristics = true
    # table_model = "tatr"
    ```

### Environment Variable

Set `KREUZBERG_LAYOUT_PRESET` to enable layout detection without modifying code:

```bash title="Terminal"
export KREUZBERG_LAYOUT_PRESET=accurate
```

Valid: `fast`, `accurate` (aliases: `yolo`, `rtdetr`, `rt-detr`).

## Table Structure Models <span class="version-badge">v4.5.3</span>

When layout detection identifies a table region, a table structure model analyzes rows, columns, headers, and spanning cells.

| Model | Config value | Size | Speed | Best for |
|-------|-------------|------|-------|----------|
| **TATR** | `"tatr"` (default) | 30 MB | Fast | General-purpose, consistent results |
| SLANeXT Wired | `"slanet_wired"` | 365 MB | Moderate | Bordered/gridlined tables |
| SLANeXT Wireless | `"slanet_wireless"` | 365 MB | Moderate | Borderless tables |
| SLANeXT Auto | `"slanet_auto"` | ~737 MB | Slower | Mixed documents (auto-classifies per page) |
| SLANet-plus | `"slanet_plus"` | 7.78 MB | Fastest | Resource-constrained environments |

!!! note "Model Download"
    SLANeXT models are not downloaded by default. Use `cache warm --all-table-models` to pre-download, or they download automatically on first use.

## GPU Acceleration

Layout detection uses ONNX Runtime with automatic provider selection:

| Provider | Platform | Notes |
|----------|----------|-------|
| CPU | All | Default, no setup needed |
| CUDA | Linux, Windows | Requires CUDA toolkit + cuDNN |
| CoreML | macOS | Automatic on Apple Silicon |
| TensorRT | Linux | Requires TensorRT |

To override:

```python
config = ExtractionConfig(
    layout=LayoutDetectionConfig(preset="accurate"),
    acceleration=AccelerationConfig(provider="cuda", device_id=0)
)
```

See [AccelerationConfig reference](../reference/configuration.md#accelerationconfig) for details.

## Layout Classes

All model backends map to 17 canonical classes:

| Class | Fast | Accurate | Description |
|-------|------|----------|-------------|
| `Caption` | Yes | Yes | Figure or table caption |
| `Footnote` | Yes | Yes | Page footnote |
| `Formula` | Yes | Yes | Mathematical formula |
| `ListItem` | Yes | Yes | List item or bullet point |
| `PageFooter` | Yes | Yes | Running page footer |
| `PageHeader` | Yes | Yes | Running page header |
| `Picture` | Yes | Yes | Image, chart, or diagram |
| `SectionHeader` | Yes | Yes | Section heading |
| `Table` | Yes | Yes | Tabular data region |
| `Text` | Yes | Yes | Body text paragraph |
| `Title` | Yes | Yes | Document or page title |
| `DocumentIndex` | — | Yes | Table of contents |
| `Code` | — | Yes | Code block |
| `CheckboxSelected` | — | Yes | Checked checkbox |
| `CheckboxUnselected` | — | Yes | Unchecked checkbox |
| `Form` | — | Yes | Form region |
| `KeyValueRegion` | — | Yes | Key-value pair region |

## Acknowledgments

- **[Docling](https://github.com/DS4SD/docling)** — RT-DETR v2 model and layout classification approach
- **[TATR](https://github.com/microsoft/table-transformer)** — Table structure recognition with ONNX
- **[PaddleOCR](https://github.com/PaddlePaddle/PaddleOCR)** — SLANeXT table structure and PP-LCNet classifier models

## Related

- [Configuration Reference](../reference/configuration.md#layoutdetectionconfig) — full field reference
- [Element-Based Output](element-based-output.md) — using layout-aware results
