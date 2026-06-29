import * as fs from "node:fs";

export async function writeRedactedPdf(filePath: string, text: string): Promise<void> {
  try {
    const { PDFDocument, StandardFonts } = await import("pdf-lib");
    const pdfDoc = await PDFDocument.create();
    const font = await pdfDoc.embedFont(StandardFonts.Helvetica);
    const fontSize = 12;
    const lines = text.split("\n");
    let pageY = 800;
    let page = pdfDoc.addPage([595.28, 841.89]);

    for (const line of lines) {
      if (pageY < 50) {
        page = pdfDoc.addPage([595.28, 841.89]);
        pageY = 800;
      }
      page.drawText(line.slice(0, 90), { x: 50, y: pageY, size: fontSize, font });
      pageY -= fontSize * 1.5;
    }

    const pdfBytes = await pdfDoc.save();
    fs.writeFileSync(filePath, pdfBytes);
  } catch {
    fs.writeFileSync(filePath, text, "utf-8");
  }
}
