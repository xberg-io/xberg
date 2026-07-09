import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getEngine } from "../engine.js";

/**
 * Public category strings accepted by this tool's Zod schema (documented,
 * human-facing form) mapped to the snake_case strings the wasm engine's
 * `PiiCategory::from(String)` recognizes (see
 * `crates/xberg/src/types/redaction.rs`). Any category not in this map falls
 * through to `PiiCategory::Custom` in the engine and will never match,
 * so we normalize here before calling into the engine.
 */
const CATEGORY_TO_ENGINE: Record<string, string> = {
  EMAIL: "email",
  PHONE: "phone",
  SSN: "ssn",
  CREDIT_CARD: "credit_card",
  IP_ADDRESS: "ip_address",
  DATE: "date_of_birth",
  PERSON: "person",
  ORGANIZATION: "organization",
  LOCATION: "location",
};

function toEngineCategories(categories?: string[]): string[] | undefined {
  if (!categories || categories.length === 0) return undefined;
  return categories.map((c) => CATEGORY_TO_ENGINE[c] ?? c.toLowerCase());
}

/** Reverse of `CATEGORY_TO_ENGINE`: engine snake_case -> public category string. */
const ENGINE_TO_CATEGORY: Record<string, string> = Object.fromEntries(
  Object.entries(CATEGORY_TO_ENGINE).map(([publicName, engineName]) => [engineName, publicName])
);

/**
 * Map an engine category back to its documented public form (e.g.
 * `date_of_birth` -> `DATE`). Falls back to an upper-cased passthrough for
 * categories with no explicit mapping so `credit_card` still reads as
 * `CREDIT_CARD`.
 */
function toPublicCategory(category: string): string {
  return ENGINE_TO_CATEGORY[category] ?? category.toUpperCase();
}

interface EnginePiiMatch {
  start: number;
  end: number;
  category: string;
  text: string;
}

interface EngineRedactResult {
  redacted: string;
  // serde_wasm_bindgen serializes the Rust HashMap as a JS `Map`, not a
  // plain object.
  rehydrationMap: Map<string, string>;
}

export function registerPiiTools(server: McpServer): void {
  server.tool(
    "detect_pii",
    "Detect PII entities in text using pattern matching. Returns array of { entity_type, text, start, end, score }. For production use with ONNX models, integrate GLiNER2-PII.",
    {
      text: z.string().describe("Text to analyze for PII"),
      categories: z
        .array(z.string())
        .optional()
        .describe("Filter to specific categories (EMAIL, PHONE, SSN, CREDIT_CARD, IP_ADDRESS, DATE, PERSON, ORGANIZATION, LOCATION)"),
    },
    async ({ text, categories }) => {
      try {
        const engine = getEngine();
        const matches = (await engine.detect_pii(text, toEngineCategories(categories))) as EnginePiiMatch[];

        const findings = matches.map((m) => ({
          entity_type: toPublicCategory(m.category),
          text: m.text,
          start: m.start,
          end: m.end,
          score: 1,
        }));

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                findings,
                total: findings.length,
              }, null, 2),
            },
          ],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `detect_pii failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "redact_document",
    "Redact PII from text using token replacement (default) or masking. Token replacement produces stable IDs like [PERSON_1], [EMAIL_2] which can be rehydrated later.",
    {
      text: z.string().describe("Text to redact"),
      strategy: z
        .enum(["token_replace", "mask", "hash"])
        .optional()
        .default("token_replace")
        .describe("Redaction strategy: token_replace (stable IDs), mask (***), hash (SHA256)"),
    },
    async ({ text, strategy }) => {
      try {
        const engine = getEngine();
        const { redacted, rehydrationMap } = (await engine.redact(text, strategy)) as EngineRedactResult;
        const token_map = Object.fromEntries(rehydrationMap);

        // Preserve the pre-migration public fields `entities_redacted` and
        // `categories` (per-category counts). The engine's redact does not
        // expose them, so recompute from a pattern scan — mirrors the old
        // detect-then-group behaviour and covers every strategy (mask/hash
        // included, not just token_replace). Engine categories are mapped back
        // to the tool's documented public form (e.g. `date_of_birth` -> `DATE`,
        // `credit_card` -> `CREDIT_CARD`).
        const matches = engine.detect_pii(text) as EnginePiiMatch[];
        const categories: Record<string, number> = {};
        for (const m of matches) {
          const cat = toPublicCategory(m.category);
          categories[cat] = (categories[cat] ?? 0) + 1;
        }

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                redacted_text: redacted,
                token_map,
                entities_redacted: matches.length,
                categories,
              }, null, 2),
            },
          ],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `redact_document failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );
}
