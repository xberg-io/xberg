import { describe, it, expect } from "vitest";
import { toParsedOcrOutput } from "../../src/lib/ocr-to-layout.js";
import type { OcrLine } from "../../src/lib/types.js";

describe("lib/ocr-to-layout", () => {
  it("maps a single OCR line to one ParsedOcrOutput block", () => {
    const lines: OcrLine[] = [{ text: "Hi", confidence: 0.9, bbox: { x: 1, y: 2, w: 3, h: 4 } }];
    const out = toParsedOcrOutput(lines);
    expect(out.chunks).toHaveLength(1);
    const chunk = out.chunks[0]!;
    expect(chunk.blocks).toHaveLength(1);
    const block = chunk.blocks[0]!;
    expect(block.content).toBe("Hi");
    expect(block.metadata.avgOcrConfidence).toBe(0.9);
    expect(block.boundingBox).toEqual({ left: 1, top: 2, right: 4, bottom: 6 });
  });
});
