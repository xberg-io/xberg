---
language: typescript
target: wasm
---

```typescript title="WASM"
import init, { WasmExtractInput, extract } from "@xberg-io/xberg-wasm";

await init();

const fileInput = document.getElementById("file") as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const result = await extract(WasmExtractInput.fromBytes(bytes, file.type || "application/octet-stream"));
  console.log(`Metadata: ${JSON.stringify(result.results[0].metadata)}`);

  // Access common metadata fields
  if (result.results[0].metadata.title) {
    console.log(`Title: ${result.results[0].metadata.title}`);
  }

  // Access format-specific metadata
  const metadata = result.results[0].metadata;

  // For HTML files
  if (metadata.format?.format_type === "html") {
    const htmlMeta = metadata.format;
    console.log(`HTML Title: ${htmlMeta.title}`);
    console.log(`Description: ${htmlMeta.description}`);

    // Access keywords as array
    if (htmlMeta.keywords && htmlMeta.keywords.length > 0) {
      console.log(`Keywords: ${htmlMeta.keywords.join(", ")}`);
    }

    // Access canonical URL
    if (htmlMeta.canonical_url) {
      console.log(`Canonical URL: ${htmlMeta.canonical_url}`);
    }

    // Access Open Graph fields
    if (htmlMeta.open_graph) {
      if (htmlMeta.open_graph["title"]) {
        console.log(`OG Title: ${htmlMeta.open_graph["title"]}`);
      }
      if (htmlMeta.open_graph["image"]) {
        console.log(`OG Image: ${htmlMeta.open_graph["image"]}`);
      }
    }

    // Access Twitter Card fields
    if (htmlMeta.twitter_card && htmlMeta.twitter_card["card"]) {
      console.log(`Twitter Card Type: ${htmlMeta.twitter_card["card"]}`);
    }

    // Access headers
    if (htmlMeta.headers && htmlMeta.headers.length > 0) {
      console.log(`Headers: ${htmlMeta.headers.map((h: any) => h.text).join(", ")}`);
    }

    // Access links
    if (htmlMeta.links && htmlMeta.links.length > 0) {
      htmlMeta.links.forEach((link: any) => {
        console.log(`Link: ${link.href} (${link.text})`);
      });
    }

    // Access images
    if (htmlMeta.images && htmlMeta.images.length > 0) {
      htmlMeta.images.forEach((image: any) => {
        console.log(`Image: ${image.src}`);
      });
    }

    // Access structured data
    if (htmlMeta.structured_data && htmlMeta.structured_data.length > 0) {
      console.log(`Structured data items: ${htmlMeta.structured_data.length}`);
    }
  }

  // PDF-specific fields are at the top level of metadata
  if (metadata.pages) {
    console.log(`Pages: ${metadata.pages.totalCount}`);
  }
  if (metadata.authors && metadata.authors.length > 0) {
    console.log(`Authors: ${metadata.authors.join(", ")}`);
  }
}
```
