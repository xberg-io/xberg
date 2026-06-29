import * as fs from "node:fs";

export function writeRedactedText(filePath: string, text: string): void {
  fs.writeFileSync(filePath, text, "utf-8");
}
