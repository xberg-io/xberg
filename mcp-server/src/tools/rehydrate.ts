import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import * as fs from "fs";
import * as path from "path";
import { getCacheDir } from "../store.js";
import { getEngine } from "../engine.js";

export function registerRehydrateTools(server: McpServer): void {
  server.tool(
    "rehydrate_tokens",
    "Apply a token map to a redacted text to restore original PII values. Obtain the map first with rehydrate_document.",
    {
      redacted_text: z.string().describe("Text with redaction tokens like [EMAIL_1]"),
      token_map: z.record(z.string()).describe("Map of token IDs to original values"),
    },
    async ({ redacted_text, token_map }) => {
      try {
        let text = redacted_text;
        for (const [token, original] of Object.entries(token_map)) {
          text = text.split(token).join(original);
        }
        return {
          content: [{ type: "text" as const, text: JSON.stringify({ rehydrated_text: text }) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `rehydrate_tokens failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "list_tokens",
    "List all redaction tokens present in a redacted text (does NOT reveal original values).",
    { redacted_text: z.string() },
    async ({ redacted_text }) => {
      try {
        const tokenPattern = /\[([A-Z_]+_\d+)\]/g;
        const tokens = new Set<string>();
        let match: RegExpExecArray | null;
        const regex = new RegExp(tokenPattern.source, tokenPattern.flags);
        while ((match = regex.exec(redacted_text)) !== null) {
          if (match[1]) tokens.add(match[1]);
        }
        const uniqueTokens = Array.from(tokens).sort();
        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                tokens: uniqueTokens,
                count: uniqueTokens.length,
                note: "Use rehydrate_document with a passphrase to get the token→value map.",
              }),
            },
          ],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `list_tokens failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "rehydrate_document",
    "Decrypt a rehydration map file and return the token→original map. Combine with rehydrate_tokens to restore full text. Map files are created by ingest_folder.",
    {
      document_id: z.string().describe("Document base name (without .map extension)"),
      passphrase: z.string().describe("Passphrase used when the map was encrypted"),
      rehydration_dir: z.string().optional().describe("Override the default rehydration map directory"),
    },
    async ({ document_id, passphrase, rehydration_dir }) => {
      try {
        const dir = rehydration_dir ?? path.join(getCacheDir(), "rehydration");
        const mapPath = path.join(dir, `${document_id}.map`);

        if (!fs.existsSync(mapPath)) {
          return {
            content: [
              {
                type: "text" as const,
                text: JSON.stringify({ error: `Rehydration map not found: ${mapPath}` }),
              },
            ],
            isError: true,
          };
        }

        const mapBytes = fs.readFileSync(mapPath);
        const engine = getEngine();
        const decrypted = engine.decrypt_map(new Uint8Array(mapBytes), passphrase);
        const tokenMap: Record<string, string> =
          decrypted instanceof Map ? Object.fromEntries(decrypted) : decrypted;
        return {
          content: [{ type: "text" as const, text: JSON.stringify({ token_map: tokenMap }) }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `rehydrate_document failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );
}
