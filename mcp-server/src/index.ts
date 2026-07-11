import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { startHttp } from "./transports/http.js";
import { registerExtractTools } from "./tools/extract.js";
import { registerCollectionTools } from "./tools/collection.js";
import { registerQueryTools } from "./tools/query.js";
import { registerDocumentTools } from "./tools/document.js";
import { registerIngestTools } from "./tools/ingest.js";
import { registerRehydrateTools } from "./tools/rehydrate.js";
import { registerPiiTools } from "./tools/pii.js";
import { registerCacheTools } from "./tools/cache.js";
import { registerReportTools } from "./tools/reports.js";
import { registerStatsTools } from "./tools/stats.js";
import { registerIntelligenceTools } from "./tools/intelligence.js";
import { registerMediaTools } from "./tools/media.js";
import { registerWebTools } from "./tools/web.js";
import { WarmupManager } from "./warmup.js";
import { initializeEngine } from "./engine.js";
import { getCacheDir } from "./paths.js";

const server = new McpServer({
  name: "xberg-mcp",
  version: "0.1.0",
});

registerExtractTools(server);
registerCollectionTools(server);
registerQueryTools(server);
registerDocumentTools(server);
registerIngestTools(server);
registerRehydrateTools(server);
registerPiiTools(server);
registerCacheTools(server);
registerReportTools(server);
registerStatsTools(server);
registerIntelligenceTools(server);
registerMediaTools(server);
registerWebTools(server);

async function main() {
  const cacheDir = getCacheDir();
  const warmup = new WarmupManager(cacheDir);
  const missing = warmup.getMissingModels();
  if (missing.length > 0) {
    console.error(`[xberg-mcp] First-time setup: downloading ${missing.join(", ")}...`);
  }

  // Build the wasm engine (B) wired to C's shared runtime factory before we
  // accept requests, so tool handlers can rely on getEngine() being ready.
  await initializeEngine();
  console.error("[xberg-mcp] engine initialized");

  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("[xberg-mcp] started");

  try {
    await startHttp(server);
  } catch (err) {
    console.error(`[xberg-mcp] HTTP transport failed to start (stdio still works): ${err instanceof Error ? err.message : String(err)}`);
  }
}

main().catch((err) => {
  console.error("[xberg-mcp] fatal:", err);
  process.exit(1);
});