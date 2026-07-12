// scripts/export-to-mcp.mjs
import { cpSync, rmSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const outDir = join(here, "..", "out");
const targetDir = join(here, "..", "..", "..", "mcp-server", "ui-dist");

if (!existsSync(outDir)) {
  throw new Error(`static export not found at ${outDir} — run "next build" first`);
}

rmSync(targetDir, { recursive: true, force: true });
cpSync(outDir, targetDir, { recursive: true });
console.log(`copied ${outDir} -> ${targetDir}`);
