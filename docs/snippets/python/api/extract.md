```python title="Python"
from xberg import ExtractInput, extract

output = await extract(ExtractInput(kind="uri", uri="document.pdf"))

print(output.results[0].content)
print(f"Results: {output.summary.results}")
```
