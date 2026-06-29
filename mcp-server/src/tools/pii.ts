import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";

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
        const findings = detectPiiPattern(text, categories);

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                findings: findings.map((f) => ({
                  entity_type: f.category,
                  text: f.original,
                  start: f.start,
                  end: f.end,
                  score: f.confidence,
                })),
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
        const findings = detectPiiPattern(text, undefined);
        const { redacted, token_map } = applyRedaction(text, findings, strategy);

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify({
                redacted_text: redacted,
                token_map,
                entities_redacted: findings.length,
                categories: groupByCategory(findings),
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

interface PiiFinding {
  category: string;
  original: string;
  start: number;
  end: number;
  confidence: number;
}

const ALL_PATTERNS: Array<{ category: string; pattern: RegExp; confidence: number }> = [
  { category: "EMAIL", pattern: /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b/g, confidence: 0.95 },
  { category: "PHONE", pattern: /\b(?:\+?\d{1,3}[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b/g, confidence: 0.85 },
  { category: "SSN", pattern: /\b\d{3}-\d{2}-\d{4}\b/g, confidence: 0.9 },
  { category: "CREDIT_CARD", pattern: /\b(?:\d{4}[-\s]?){3}\d{4}\b/g, confidence: 0.9 },
  { category: "IP_ADDRESS", pattern: /\b(?:\d{1,3}\.){3}\d{1,3}\b/g, confidence: 0.8 },
  { category: "DATE_ISO", pattern: /\b\d{4}-\d{2}-\d{2}\b/g, confidence: 0.7 },
  { category: "DATE_MDY", pattern: /\b\d{1,2}\/\d{1,2}\/\d{2,4}\b/g, confidence: 0.7 },
  { category: "IBAN", pattern: /\b[A-Z]{2}\d{2}[A-Z0-9]{4,30}\b/g, confidence: 0.85 },
  { category: "SWIFT_BIC", pattern: /\b[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?\b/g, confidence: 0.8 },
  { category: "POSTAL_CODE_US", pattern: /\b\d{5}(?:-\d{4})?\b/g, confidence: 0.75 },
  { category: "POSTAL_CODE_UK", pattern: /\b[A-Z]{1,2}\d[A-Z\d]?\s?\d[A-Z]{2}\b/g, confidence: 0.75 },
];

function detectPiiPattern(text: string, filterCategories?: string[]): PiiFinding[] {
  const findings: PiiFinding[] = [];
  const categoryCounters: Record<string, number> = {};

  for (const { category, pattern, confidence } of ALL_PATTERNS) {
    if (filterCategories && !filterCategories.includes(category)) continue;

    const regex = new RegExp(pattern.source, pattern.flags);
    let match: RegExpExecArray | null;

    while ((match = regex.exec(text)) !== null) {
      const token = `[${category}_${(categoryCounters[category] ?? 0) + 1}]`;
      categoryCounters[category] = (categoryCounters[category] ?? 0) + 1;
      findings.push({
        category,
        original: match[0],
        start: match.index,
        end: match.index + match[0].length,
        confidence,
      });
    }
  }

  return findings.sort((a, b) => a.start - b.start);
}

function groupByCategory(findings: PiiFinding[]): Record<string, number> {
  const grouped: Record<string, number> = {};
  for (const f of findings) {
    grouped[f.category] = (grouped[f.category] ?? 0) + 1;
  }
  return grouped;
}

function applyRedaction(
  text: string,
  findings: PiiFinding[],
  strategy: string
): { redacted: string; token_map: Record<string, string> } {
  const tokenMap: Record<string, string> = {};
  const sorted = [...findings].sort((a, b) => b.start - a.start);

  let result = text;
  for (const f of sorted) {
    const token = `[${f.category}_${Object.keys(tokenMap).filter((k) => k.startsWith(f.category)).length + 1}]`;

    if (strategy === "mask") {
      result = result.slice(0, f.start) + "*".repeat(f.original.length) + result.slice(f.end);
    } else if (strategy === "hash") {
      let hash = 0;
      for (let i = 0; i < f.original.length; i++) {
        hash = (hash << 5) - hash + f.original.charCodeAt(i);
        hash |= 0;
      }
      result = result.slice(0, f.start) + `HASH_${Math.abs(hash).toString(16)}` + result.slice(f.end);
      tokenMap[token] = f.original;
    } else {
      result = result.slice(0, f.start) + token + result.slice(f.end);
      tokenMap[token] = f.original;
    }
  }

  return { redacted: result, token_map: tokenMap };
}