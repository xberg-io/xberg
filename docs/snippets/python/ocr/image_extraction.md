```python title="Python"
from kreuzberg import extract_file_sync, ExtractionConfig, ImageExtractionConfig

config: ExtractionConfig = ExtractionConfig(
    images=ImageExtractionConfig(
        extract_images=True,
        target_dpi=200,
        max_image_dimension=2048,
        inject_placeholders=True,  # set to False to extract images without markdown references
        auto_adjust_dpi=True,
    )
)

result = extract_file_sync("document.pdf", config=config)

print(f"Content length: {len(result.content)} characters")
```
