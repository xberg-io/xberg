```python title="Python"
from xberg import ExtractInput, extract, ExtractionConfig, ImageExtractionConfig

config: ExtractionConfig = ExtractionConfig(
    images=ImageExtractionConfig(
        extract_images=True,
        target_dpi=200,
        max_image_dimension=2048,
        inject_placeholders=True,  # set to False to extract images without markdown references
        auto_adjust_dpi=True,
    )
)

result = extract(ExtractInput.from_uri("document.pdf"), config)

print(f"Content length: {len(result.results[0].content)} characters")
```
