```python title="Document Structure Config (Python)"
from xberg import ExtractInput, extract, ExtractionConfig

# Enable document structure output
config = ExtractionConfig(include_document_structure=True)

result = extract(ExtractInput.from_uri("document.pdf"), config)

# Access the document tree
if result.document:
    for node in result.document["nodes"]:
        node_type = node["content"]["node_type"]
        text = node["content"].get("text", "")
        print(f"[{node_type}] {text[:80]}")
```
