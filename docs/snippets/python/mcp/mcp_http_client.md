```python title="Python"
import asyncio

import httpx
from mcp import ClientSession
from mcp.client.streamable_http import streamable_http_client

MCP_URL = "http://127.0.0.1:8001/mcp"


async def main() -> None:
    # Requires MCP server running with HTTP transport:
    # xberg mcp --transport http --host 127.0.0.1 --port 8001

    async with httpx.AsyncClient(follow_redirects=True) as http_client:
        async with streamable_http_client(MCP_URL, http_client=http_client) as (
            read,
            write,
        ):
            async with ClientSession(read, write) as session:
                await session.initialize()

                tools = await session.list_tools()
                tool_names: list[str] = [t.name for t in tools.tools]
                print(f"Available tools: {tool_names}")

                result = await session.call_tool(
                    "extract",
                    arguments={"path": "document.pdf"},
                )
                print(result)


asyncio.run(main())
```
