```python title="Python"
from xberg import extract_sync, ExtractionConfig, PdfConfig, HierarchyConfig

config: ExtractionConfig = ExtractionConfig(
    pdf_options=PdfConfig(
        extract_metadata=True,
        hierarchy=HierarchyConfig(
            enabled=True,
            k_clusters=6,
            include_bbox=True,
            ocr_coverage_threshold=0.8
        )
    )
)

result = extract_sync("document.pdf", config=config)

# Access hierarchy information
for page in result.pages or []:
    print(f"Page {page.page_number}:")
    print(f"  Content: {page.content[:100]}...")
```
