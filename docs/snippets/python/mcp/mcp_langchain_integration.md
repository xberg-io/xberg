```python title="Python"
from langchain.agents import initialize_agent, AgentType
from langchain.tools import Tool
from langchain_openai import ChatOpenAI
import subprocess
import json

mcp_process = subprocess.Popen(
    ["xberg", "mcp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)

def extract(path: str) -> str:
    request: dict = {
        "method": "tools/call",
        "params": {
            "name": "extract",
            "arguments": {"path": path, "async": True},
        },
    }
    mcp_process.stdin.write(json.dumps(request).encode() + b"\n")
    mcp_process.stdin.flush()
    response = mcp_process.stdout.readline()
    return json.loads(response)["result"]["content"]

tools: list[Tool] = [
    Tool(name="extract_document", func=extract, description="Extract")
]

llm = ChatOpenAI(temperature=0)
agent = initialize_agent(
    tools, llm, agent=AgentType.ZERO_SHOT_REACT_DESCRIPTION
)
```
