```python title="Python"
from xberg import ExtractInput, extract_batch

inputs = [
    ExtractInput(kind="uri", uri="document.pdf"),
    ExtractInput(
        kind="bytes",
        bytes=b"Hello from memory",
        mime_type="text/plain",
        filename="note.txt",
    ),
]

output = await extract_batch(inputs)

for result in output.results:
    print(result.content[:200])
```
