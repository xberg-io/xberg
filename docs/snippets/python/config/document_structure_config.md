```python title="Document Structure Config (Python)"
from xberg import extract_sync, ExtractionConfig

# Enable document structure output
config = ExtractionConfig(include_document_structure=True)

result = extract_sync("document.pdf", config=config)

# Access the document tree
if result.document:
    for node in result.document["nodes"]:
        node_type = node["content"]["node_type"]
        text = node["content"].get("text", "")
        print(f"[{node_type}] {text[:80]}")
```
