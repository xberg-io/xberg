"use client";

import { Card, CardHeader, CardContent } from "@/components/ui/card.js";
import { PDFViewer } from "@/components/ui/pdf-viewer.js";
import { DocxViewerPreview } from "@/components/ui/docx-viewer.js";
import { XlsxViewerPreview } from "@/components/ui/xlsx-viewer.js";
import { BoundingBoxCitations } from "@/components/BoundingBoxCitations.js";
import { LayoutBlocks } from "@/components/LayoutBlocks.js";
import type { OcrLine } from "@/lib/types.js";

const DOCX_MIME =
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const XLSX_MIME =
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

function isSpreadsheetMime(mime: string | undefined): boolean {
  if (!mime) return false;
  return (
    mime === XLSX_MIME ||
    mime === "application/vnd.ms-excel" ||
    mime.endsWith(".sheet") ||
    mime.includes("spreadsheet")
  );
}

export interface DocumentViewerProps {
  /** Object/file URL for the extend viewer. When absent the viewer shows its
   *  built-in empty/loading state — never invent a server URL here. */
  fileUrl?: string;
  mime?: string;
  redactedText: string;
  counts: Record<string, number>;
  /** Clear PII values keyed by token. MUST NEVER be rendered (PII safety). */
  map?: Record<string, string>;
  /** OCR layout lines; when present, the layout overlay panel is rendered. */
  layoutLines?: OcrLine[];
}

export function DocumentViewer({
  fileUrl,
  mime,
  redactedText,
  counts,
  map,
  layoutLines,
}: DocumentViewerProps) {
  return (
    <div className="flex flex-col gap-4">
      <div className="min-h-[640px] overflow-hidden rounded border bg-background">
        {mime === "application/pdf" ? (
          <PDFViewer src={fileUrl} />
        ) : mime === DOCX_MIME ? (
          <DocxViewerPreview
            src={fileUrl}
            isDark={false}
            onIsDarkChange={() => {}}
          />
        ) : isSpreadsheetMime(mime) ? (
          <XlsxViewerPreview
            src={fileUrl}
            isDark={false}
            onIsDarkChange={() => {}}
          />
        ) : (
          <div className="grid h-[640px] place-items-center p-6 text-center">
            <p className="text-sm text-muted-foreground">
              No preview available for this document type.
            </p>
          </div>
        )}
      </div>

      {layoutLines && layoutLines.length > 0 ? (
        <Card>
          <CardHeader>
            <h2 className="text-base font-semibold">Layout</h2>
          </CardHeader>
          <CardContent>
            <LayoutBlocks lines={layoutLines} />
          </CardContent>
        </Card>
      ) : null}

      <Card>
        <CardContent>
          <BoundingBoxCitations
            redactedText={redactedText}
            counts={counts}
            map={map}
          />
        </CardContent>
      </Card>
    </div>
  );
}
