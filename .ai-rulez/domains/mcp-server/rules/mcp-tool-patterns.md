---
priority: high
---

# MCP Tool Patterns

- Every tool must define its input schema with Zod and call `z.parse()` before any other logic
- Tool registration: `server.tool(name, description, zodSchema, handler)` — name must be snake_case
- Tool names are stable public API — renaming a tool is a breaking change for all connected agents
- Handlers must return `{ content: [{ type: "text", text: string }] }` — never return raw objects
- For structured output, serialize to JSON string inside the `text` field; agents parse it client-side
- Errors: catch all thrown errors and return `{ isError: true, content: [{ type: "text", text: msg }] }`
- Never call `process.exit()` from a tool handler — let the MCP SDK manage the server lifecycle
- Native binding calls (`xberg.*`, `openSqlite`, `embedTexts`) must be awaited; never fire-and-forget
- Long-running tools (ingest_folder, export_collection) must support cancellation via `AbortSignal` when SDK provides it
- Log tool invocations at `DEBUG` level with tool name + key params; never log PII or file contents
- Keep tool files small: one file per tool category (extract.ts, ingest.ts, etc.), max ~200 lines
- Add new tools to the `registerXxxTools` call in `index.ts` — missing registration means the tool is invisible to agents
