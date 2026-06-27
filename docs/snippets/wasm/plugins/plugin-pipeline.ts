import type { ExtractedDocument } from "@xberg-io/xberg-wasm";
import { extract, initWasm } from "@xberg-io/xberg-wasm";

interface Plugin {
  name: string;
  execute: (result: ExtractedDocument) => Promise<ExtractedDocument>;
}

class TextCleanerPlugin implements Plugin {
  name = "text-cleaner";

  async execute(result: ExtractedDocument): Promise<ExtractedDocument> {
    const cleaned = result.content.replace(/\x00/g, "").replace(/\s+/g, " ").trim();

    return { ...result, content: cleaned };
  }
}

class MetadataEnricherPlugin implements Plugin {
  name = "metadata-enricher";

  async execute(result: ExtractedDocument): Promise<ExtractedDocument> {
    return {
      ...result,
      metadata: {
        ...result.metadata,
        processedAt: new Date().toISOString(),
        contentLength: result.content.length,
      },
    };
  }
}

async function executePipeline(
  bytes: Uint8Array,
  mimeType: string,
  plugins: Plugin[],
): Promise<ExtractedDocument> {
  await initWasm();

  const output = await extract({ kind: "bytes", bytes, mimeType: mimeType });
  let result = output.results[0];
  if (!result) {
    throw new Error("No document extracted");
  }

  for (const plugin of plugins) {
    console.log(`Executing plugin: ${plugin.name}`);
    result = await plugin.execute(result);
  }

  return result;
}

const pipeline = [new TextCleanerPlugin(), new MetadataEnricherPlugin()];

executePipeline(new Uint8Array([1, 2, 3]), "application/pdf", pipeline)
  .then((r) => console.log("Pipeline complete", r))
  .catch(console.error);
