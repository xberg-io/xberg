```python title="Python"
from xberg import ExtractInput, extract, ExtractionConfig, ExtractedTable

result = extract(ExtractInput.from_uri("document.pdf"), ExtractionConfig())

for table in result.results[0].tables:
    row_count: int = len(table.cells)
    print(f"Table with {row_count} rows")
    print(table.markdown)
    for row in table.cells:
        print(row)
```
