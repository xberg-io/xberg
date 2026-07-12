import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { createUiRoutes } from "../http/ui-server.js";
import { extractToken, isValidToken } from "../http/auth.js";

const DEFAULT_PORT = Number(process.env["XBERG_MCP_PORT"] ?? 8080);
const DEFAULT_HOST = process.env["XBERG_MCP_HOST"] ?? "127.0.0.1";
const MAX_MESSAGE_BODY_BYTES = 10 * 1024 * 1024; // 10 MiB

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
  const uiToken = ui.token;

  const httpServer = createServer(async (req: IncomingMessage, res: ServerResponse) => {
    try {
      const url = new URL(req.url ?? "/", `http://${host}`);

      if (req.method === "GET" && url.pathname === "/sse") {
        // Token arrives via `?token=` here (not just Authorization) because
        // EventSource can't set custom headers. This can leak into access
        // logs, proxy logs, or browser history — acceptable for the
        // localhost-only default, but scrub tokens from any access logging
        // added later if this server is ever exposed beyond 127.0.0.1.
        const candidate = extractToken(req, url);
        if (!isValidToken(candidate, uiToken)) {
          res.writeHead(401, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "unauthorized" }));
          return;
        }
        const transport = new SSEServerTransport("/message", res);
        sessions.set(transport.sessionId, transport);
        res.on("close", () => sessions.delete(transport.sessionId));
        await server.connect(transport);
        return;
      }

      if (req.method === "POST" && url.pathname === "/message") {
        const candidate = extractToken(req, url);
        if (!isValidToken(candidate, uiToken)) {
          res.writeHead(401, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "unauthorized" }));
          return;
        }
        const sessionId = url.searchParams.get("sessionId") ?? "";
        const transport = sessions.get(sessionId);
        if (!transport) {
          res.writeHead(404).end("Unknown session");
          return;
        }
        const chunks: Buffer[] = [];
        let totalBytes = 0;
        for await (const chunk of req) {
          const buf = chunk as Buffer;
          totalBytes += buf.length;
          if (totalBytes > MAX_MESSAGE_BODY_BYTES) {
            res.writeHead(413, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "payload too large" }));
            req.resume();
            return;
          }
          chunks.push(buf);
        }
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
    } catch (err) {
      if (!res.headersSent) {
        const msg = err instanceof Error ? err.message : String(err);
        res.writeHead(500, { "Content-Type": "application/json" }).end(JSON.stringify({ error: msg }));
      } else {
        res.end();
      }
    }
  });

  await new Promise<void>((resolve, reject) => {
    httpServer.once("error", reject);
    httpServer.listen(port, host, () => {
      httpServer.off("error", reject);
      resolve();
    });
  });
  const address = httpServer.address();
  const actualPort = address !== null && typeof address !== "string" ? address.port : port;

  process.stderr.write(`[xberg-mcp] HTTP/SSE transport started on http://${host}:${actualPort}/sse\n`);
  const uiUrl = `http://${host}:${actualPort}/ui`;
  if (process.env["XBERG_MCP_LOG_UI_TOKEN"] === "1") {
    process.stderr.write(`[xberg-mcp] UI available at ${uiUrl}?token=${ui.token}\n`);
  } else {
    process.stderr.write(`[xberg-mcp] UI available at ${uiUrl}?token=<redacted>\n`);
  }

  return {
    port: actualPort,
    uiToken: ui.token,
    close: () => new Promise<void>((resolve, reject) => httpServer.close((err) => (err ? reject(err) : resolve()))),
  };
}
