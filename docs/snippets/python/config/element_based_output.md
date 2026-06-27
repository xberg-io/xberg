```python title="Element-Based Output (Python)"
from xberg import extract_sync, ExtractionConfig

# Configure element-based output
config = ExtractionConfig(result_format="element_based")

# Extract document
result = extract_sync("document.pdf", config=config)

# Access elements
for element in result.elements:
    print(f"Type: {element.element_type}")
    print(f"Text: {element.text[:100]}")

    if element.metadata.page_number:
        print(f"Page: {element.metadata.page_number}")

    if element.metadata.coordinates:
        coords = element.metadata.coordinates
        print(f"Coords: ({coords.left}, {coords.top}) - ({coords.right}, {coords.bottom})")

    print("---")

# Filter by element type
titles = [e for e in result.elements if e.element_type == "title"]
for title in titles:
    level = title.metadata.additional.get("level", "unknown")
    print(f"[{level}] {title.text}")
```
