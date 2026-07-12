import { Card, CardHeader, CardContent } from "@/components/ui/card.js";
import { Badge } from "@/components/ui/badge.js";
import type { IngestHistoryEntry } from "@/lib/types.js";

/**
 * V1: redacted text + PII counts only. Lot 3 replaces the body with
 * extend-hq PDF/DOCX/XLSX viewers, `LayoutBlocks`, and
 * `BoundingBoxCitations` — keep this component's name and the
 * `{ entry }` prop shape stable for that migration.
 */
export function DocumentViewer({ entry }: { entry: IngestHistoryEntry }) {
  return (
    <Card>
      <CardHeader>
        <h1 className="text-lg font-semibold">{entry.filename}</h1>
        <div className="mt-1 flex gap-1">
          {Object.entries(entry.piiCategoryCounts).map(([cat, n]) => (
            <Badge key={cat}>
              {cat}: {n}
            </Badge>
          ))}
        </div>
      </CardHeader>
      <CardContent>
        <p className="whitespace-pre-wrap text-sm">{entry.redactedText}</p>
      </CardContent>
    </Card>
  );
}
