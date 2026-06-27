import type { ExtractedDocument } from "@xberg-io/xberg-wasm";
import { extract, initWasm } from "@xberg-io/xberg-wasm";

class MarkdownFormatter {
  async process(result: ExtractedDocument): Promise<ExtractedDocument> {
    const formatted = result.content.replace(/^(.+)$/gm, "# $1").replace(/\n\n+/g, "\n\n");

    return {
      ...result,
      content: formatted,
    };
  }

  getName(): string {
    return "markdown-formatter";
  }

  getVersion(): string {
    return "1.0.0";
  }
}

async function demonstrateCustomProcessor() {
  await initWasm();

  const processor = new MarkdownFormatter();
  const bytes = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

  let result = await extract({ kind: "bytes", bytes, mimeType: "application/pdf" });

  result = await processor.process(result);
  console.log("Formatted result:", result.content);

  return result;
}

demonstrateCustomProcessor().catch(console.error);
