import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { createUiRoutes } from "../http/ui-server.js";

const DEFAULT_PORT = Number(process.env["XBERG_MCP_PORT"] ?? 8080);
const DEFAULT_HOST = process.env["XBERG_MCP_HOST"] ?? "127.0.0.1";

export interface HttpHandle {
  port: number;
  uiToken: string;
  close(): Promise<void>;
}

export async function startHttp(
  server: McpServer,
  options: { host?: string; port?: number } = {},
): Promise<HttpHandle> {
  const host = options.host ?? DEFAULT_HOST;
  const port = options.port ?? DEFAULT_PORT;

  // SSE transport: each GET /sse opens a session; POST /message sends a message
  // Requires @modelcontextprotocol/sdk >= 1.0 with SSEServerTransport
  let SSEServerTransport: new (path: string, res: ServerResponse) => import("@modelcontextprotocol/sdk/server/sse.js").SSEServerTransport;
  try {
    const mod = await import("@modelcontextprotocol/sdk/server/sse.js");
    SSEServerTransport = mod.SSEServerTransport;
  } catch {
    process.stderr.write("[xberg-mcp] HTTP transport requires @modelcontextprotocol/sdk >= 1.0 with SSE support\n");
    throw new Error("SSE transport unavailable");
  }

  const sessions = new Map<string, InstanceType<typeof SSEServerTransport>>();
  const ui = createUiRoutes();

  const httpServer = createServer(async (req: IncomingMessage, res: ServerResponse) => {
    const url = new URL(req.url ?? "/", `http://${host}`);

    if (req.method === "GET" && url.pathname === "/sse") {
      const transport = new SSEServerTransport("/message", res);
      sessions.set(transport.sessionId, transport);
      res.on("close", () => sessions.delete(transport.sessionId));
      await server.connect(transport);
      return;
    }

    if (req.method === "POST" && url.pathname === "/message") {
      const sessionId = url.searchParams.get("sessionId") ?? "";
      const transport = sessions.get(sessionId);
      if (!transport) {
        res.writeHead(404).end("Unknown session");
        return;
      }
      const chunks: Buffer[] = [];
      for await (const chunk of req) chunks.push(chunk as Buffer);
      await transport.handlePostMessage(req, res, Buffer.concat(chunks));
      return;
    }

    if (req.method === "GET" && url.pathname === "/health") {
      res.writeHead(200, { "Content-Type": "application/json" })
        .end(JSON.stringify({ status: "ok", server: "xberg-mcp" }));
      return;
    }

    if (await ui.handleRequest(req, res, url)) return;

    res.writeHead(404).end("Not Found");
  });

  await new Promise<void>((resolve) => httpServer.listen(port, host, resolve));
  const address = httpServer.address();
  const actualPort = address !== null && typeof address !== "string" ? address.port : port;

  process.stderr.write(`[xberg-mcp] HTTP/SSE transport started on http://${host}:${actualPort}/sse\n`);
  process.stderr.write(`[xberg-mcp] UI available at http://${host}:${actualPort}/ui?token=${ui.token}\n`);

  return {
    port: actualPort,
    uiToken: ui.token,
    close: () => new Promise<void>((resolve, reject) => httpServer.close((err) => (err ? reject(err) : resolve()))),
  };
}
