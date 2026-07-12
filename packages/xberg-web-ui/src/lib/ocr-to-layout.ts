import type { OcrLine } from "./types.js";
import type { ParsedOcrOutput } from "@/components/ui/layout-blocks";

export function toParsedOcrOutput(
  lines: OcrLine[],
  width = 1000,
  height = 1400
): ParsedOcrOutput {
  return {
    chunks: [
      {
        blocks: lines.map((l, i) => ({
          id: `block-${i}`,
          type: "text",
          content: l.text,
          metadata: {
            page: { number: 1, width, height },
            avgOcrConfidence: l.confidence,
          },
          boundingBox: l.bbox
            ? {
                left: l.bbox.x,
                top: l.bbox.y,
                right: l.bbox.x + l.bbox.w,
                bottom: l.bbox.y + l.bbox.h,
              }
            : { left: 0, top: 0, right: width, bottom: height },
        })),
      },
    ],
  };
}
