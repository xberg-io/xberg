import { createHash } from "node:crypto";
import type { PiiFinding } from "./detect.js";

export interface RedactionResult {
  redacted: string;
  token_map: Record<string, string>;
}

export function applyRedaction(
  text: string,
  findings: PiiFinding[],
  strategy: "token_replace" | "mask" | "hash",
): RedactionResult {
  const tokenMap: Record<string, string> = {};
  const sorted = [...findings].sort((a, b) => b.start - a.start);

  let result = text;
  for (const f of sorted) {
    if (strategy === "mask") {
      result = result.slice(0, f.start) + "*".repeat(f.original.length) + result.slice(f.end);
    } else if (strategy === "hash") {
      const hash = createHash("sha256").update(f.original).digest("hex").slice(0, 8);
      const replacement = `HASH_${hash}`;
      result = result.slice(0, f.start) + replacement + result.slice(f.end);
      tokenMap[f.token] = f.original;
    } else {
      result = result.slice(0, f.start) + f.token + result.slice(f.end);
      tokenMap[f.token] = f.original;
    }
  }

  return { redacted: result, token_map: tokenMap };
}

export function redactToString(
  text: string,
  findings: PiiFinding[],
  strategy: "token_replace" | "mask" | "hash",
): string {
  return applyRedaction(text, findings, strategy).redacted;
}
