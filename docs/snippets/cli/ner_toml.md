```toml title="kreuzberg.toml"
[ner]
backend = "llm"
custom_labels = ["Treatment", "Vessel", "Product"]

[ner.llm]
model = "openai/gpt-4o-mini"
```
