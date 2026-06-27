import type { ExtractionConfig } from "@xberg-io/xberg-wasm";
import { extract, initWasm } from "@xberg-io/xberg-wasm";

async function extractWithChunking() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("book.pdf").then((r) => r.arrayBuffer()));

  const config: ExtractionConfig = {
    chunking: {
      maxChars: 800,
      chunkOverlap: 150,
      splitOnNewlines: true,
      splitOnSentences: true,
    },
  };

  // Example: prepend heading context so each chunk carries its heading breadcrumb
  const configWithHeadings: ExtractionConfig = {
    chunking: {
      chunkerType: "markdown",
      maxChars: 800,
      prependHeadingContext: true,
    },
  };

  const result = await extract(bytes, "application/pdf", config);

  if (result.chunks) {
    console.log(`Total chunks: ${result.chunks.length}`);

    result.chunks.slice(0, 3).forEach((chunk, i) => {
      console.log(`\nChunk ${i}:`);
      console.log(`Chars: ${chunk.metadata.charStart}-${chunk.metadata.charEnd}`);
      console.log(`Content: ${chunk.content.substring(0, 100)}...`);
    });
  }
}

async function extractWithPrependHeadingContext() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("document.md").then((r) => r.arrayBuffer()));

  const config: ExtractionConfig = {
    chunking: {
      chunkerType: "markdown",
      maxChars: 800,
      prependHeadingContext: true,
    },
  };

  const result = await extract(bytes, "text/markdown", config);

  if (result.chunks) {
    console.log(`Total chunks: ${result.chunks.length}`);

    result.chunks.slice(0, 3).forEach((chunk, i) => {
      // Each chunk's content is prefixed with its heading breadcrumb
      console.log(`\nChunk ${i}: ${chunk.content.substring(0, 100)}...`);
    });
  }
}

extractWithChunking().catch(console.error);
extractWithPrependHeadingContext().catch(console.error);
