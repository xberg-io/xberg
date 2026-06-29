```python title="Python"
import asyncio
import json
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

async def main() -> None:
    server_params: StdioServerParameters = StdioServerParameters(
        command="xberg", args=["mcp"]
    )

    inputs: list[dict[str, str]] = [
        {"kind": "uri", "uri": "file1.pdf"},
        {"kind": "uri", "uri": "file2.docx"},
        {"kind": "uri", "uri": "notes.md"},
    ]
    config: dict[str, bool] = {"use_cache": True}

    async with stdio_client(server_params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()

            result = await session.call_tool(
                "extract_batch",
                arguments={"inputs": inputs, "config": config},
            )

            payload_text: str = result.content[0].text
            batch: dict = json.loads(payload_text)

            print(f"Extracted {batch['summary']['results']} files")
            for index, item in enumerate(batch["results"], start=1):
                mime_type: str | None = item.get("mime_type")
                preview: str = item["content"][:80].replace("\n", " ")
                print(f"  [{index}] {mime_type or 'unknown'}: {preview}...")

asyncio.run(main())
```
