import * as fs from "node:fs";

export async function writeRedactedDocx(filePath: string, text: string): Promise<void> {
  try {
    const { Document, Packer, Paragraph, TextRun } = await import("docx");
    const doc = new Document({
      sections: [
        {
          children: text.split("\n").map(
            (line) => new Paragraph({ children: [new TextRun(line)] })
          ),
        },
      ],
    });
    const buffer = await Packer.toBuffer(doc);
    fs.writeFileSync(filePath, buffer);
  } catch {
    fs.writeFileSync(filePath, text, "utf-8");
  }
}
