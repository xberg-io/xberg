import type { OcrLine } from "./types.js";
import type { ParsedOcrOutput } from "@/components/ui/layout-blocks";

// `getOcrBlocks` (components/ui/layout-blocks.tsx) flattens every chunk's
// blocks into one array and reads page identity purely from each block's
// own `metadata.page` — so a single chunk is sufficient here; what matters
// is that each block carries the right page number/dimensions. Each line's
// own `page` is used when present (real multi-page geometry), falling back
// to page 1 with the caller-supplied `width`/`height` when absent — which
// is every line today, since nothing yet splits a document into per-page
// images before OCR (see the doc comment on `OcrLine.page` in lib/types.ts).
export function toParsedOcrOutput(
  lines: OcrLine[],
  width = 1000,
  height = 1400
): ParsedOcrOutput {
  return {
    chunks: [
      {
        blocks: lines.map((l, i) => {
          const pageNumber = l.page?.number ?? 1;
          const pageWidth = l.page?.width ?? width;
          const pageHeight = l.page?.height ?? height;
          return {
            id: `block-${i}`,
            type: "text",
            content: l.text,
            metadata: {
              page: { number: pageNumber, width: pageWidth, height: pageHeight },
              avgOcrConfidence: l.confidence,
            },
            boundingBox: l.bbox
              ? {
                  left: l.bbox.x,
                  top: l.bbox.y,
                  right: l.bbox.x + l.bbox.w,
                  bottom: l.bbox.y + l.bbox.h,
                }
              : { left: 0, top: 0, right: pageWidth, bottom: pageHeight },
          };
        }),
      },
    ],
  };
}
