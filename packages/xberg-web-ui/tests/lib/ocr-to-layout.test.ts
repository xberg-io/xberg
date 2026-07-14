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

  it("uses each line's own page identity when present, defaulting to page 1 otherwise", () => {
    const lines: OcrLine[] = [
      { text: "Page one line", confidence: 0.9, page: { number: 1, width: 800, height: 1000 } },
      { text: "Page two line", confidence: 0.8, page: { number: 2, width: 800, height: 1000 } },
      { text: "No page info", confidence: 0.7 },
    ];
    const out = toParsedOcrOutput(lines);
    expect(out.chunks).toHaveLength(1);
    const blocks = out.chunks[0]!.blocks;
    expect(blocks[0]!.metadata.page).toEqual({ number: 1, width: 800, height: 1000 });
    expect(blocks[1]!.metadata.page).toEqual({ number: 2, width: 800, height: 1000 });
    expect(blocks[2]!.metadata.page).toEqual({ number: 1, width: 1000, height: 1400 });
  });
});
