"use client";

import { useMemo } from "react";
import { OcrBlocksPanel, getOcrBlocks } from "@/components/ui/layout-blocks";
import { toParsedOcrOutput } from "@/lib/ocr-to-layout.js";
import type { OcrLine } from "@/lib/types.js";

export interface LayoutBlocksProps {
  lines: OcrLine[];
  width?: number;
  height?: number;
  file?: string;
}

export function LayoutBlocks({
  lines,
  width = 1000,
  height = 1400,
  file,
}: LayoutBlocksProps) {
  const output = useMemo(
    () => toParsedOcrOutput(lines, width, height),
    [lines, width, height]
  );

  if (file) {
    return (
      <OcrBlocksPanel
        blocks={getOcrBlocks(output)}
        className="h-[720px]"
      />
    );
  }

  return (
    <div data-testid="layout-stack" className="grid gap-1">
      {lines.map((l, i) => (
        <div
          key={i}
          data-testid="layout-block"
          className="border rounded p-1 text-sm bg-muted/40"
        >
          <span className="font-mono">{l.text}</span>{" "}
          <span className="text-muted-foreground">
            {(l.confidence * 100).toFixed(0)}%
          </span>
        </div>
      ))}
    </div>
  );
}
