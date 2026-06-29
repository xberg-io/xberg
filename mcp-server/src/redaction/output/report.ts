import * as fs from "node:fs";
import type { PiiFinding } from "../detect.js";
import { groupByCategory } from "../detect.js";

export async function writeReport(
  reportPath: string,
  filename: string,
  findings: PiiFinding[],
): Promise<void> {
  try {
    const { Document, Packer, Paragraph, TextRun, Table, TableRow, TableCell } = await import("docx");
    const categoryCounts = groupByCategory(findings);

    const headerRow = new TableRow({
      children: [
        new TableCell({ children: [new Paragraph({ children: [new TextRun({ text: "Category", bold: true })] })] }),
        new TableCell({ children: [new Paragraph({ children: [new TextRun({ text: "Count", bold: true })] })] }),
      ],
    });

    const dataRows = Object.entries(categoryCounts).map(
      ([cat, count]) => new TableRow({
        children: [
          new TableCell({ children: [new Paragraph({ children: [new TextRun(cat)] })] }),
          new TableCell({ children: [new Paragraph({ children: [new TextRun(String(count))] })] }),
        ],
      })
    );

    const entityLines = findings.slice(0, 100).map(
      (f) => new Paragraph({ children: [new TextRun(`  ${f.token} → ${f.original} (${f.category})`)] })
    );

    const doc = new Document({
      sections: [{
        children: [
          new Paragraph({ children: [new TextRun({ text: `PII Report: ${filename}`, bold: true, size: 28 })] }),
          new Paragraph({ children: [new TextRun(`Generated: ${new Date().toISOString()}`)] }),
          new Paragraph({ children: [new TextRun(`Total PII entities: ${findings.length}`)] }),
          new Paragraph({ children: [new TextRun({ text: "Category Breakdown:", bold: true })] }),
          ...(dataRows.length > 0
            ? [new Table({ rows: [headerRow, ...dataRows] })]
            : [new Paragraph({ children: [new TextRun("No PII detected")] })]),
          new Paragraph({ children: [new TextRun({ text: "Detected Entities:", bold: true })] }),
          ...entityLines,
        ],
      }],
    });

    const buffer = await Packer.toBuffer(doc);
    fs.writeFileSync(reportPath, buffer);
  } catch {
    const categoryCounts = groupByCategory(findings);
    const lines = [
      `PII Report: ${filename}`,
      `Generated: ${new Date().toISOString()}`,
      `Total PII: ${findings.length}`,
      "",
      ...Object.entries(categoryCounts).map(([k, v]) => `${k}: ${v}`),
    ];
    fs.writeFileSync(reportPath, lines.join("\n"), "utf-8");
  }
}
