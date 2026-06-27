```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const fileInput = document.getElementById("file") as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
  const bytes = new Uint8Array(await file.arrayBuffer());

  // Build a comprehensive extraction config
  const config = {
    use_cache: true,
    enable_quality_processing: true,
    output_format: "markdown",
    include_document_structure: true,

    // Chunking configuration
    chunking: {
      strategy: "semantic",
      max_chunk_size: 1024,
      overlap: 100,
    },

    // Image extraction configuration
    images: {
      extract_images: true,
      extract_base64: false,
      extract_raw_bytes: false,
    },

    // OCR configuration
    ocr: {
      backend: "tesseract",
      languages: ["eng"],
      enabled: true,
    },

    // HTML-specific extraction options
    html_options: "article, main, .content",

    // PDF-specific options
    pdf_options: {
      ocr_strategy: "auto",
      preserve_images: true,
    },

    // Security limits
    security_limits: {
      max_archive_size: 524288000,
      max_file_count: 10000,
      max_compression_ratio: 100,
    },
  };

  try {
    const result = await extract(bytes, file.type || "application/octet-stream", config);
    console.log(`Content: ${result.content.substring(0, 100)}...`);
    console.log(`Language: ${result.metadata?.language ?? "Unknown"}`);
    console.log(`Chunks: ${result.chunks?.length ?? 0}`);
    if (result.images?.length) {
      console.log(`Images: ${result.images.length}`);
    }
  } catch (err) {
    console.error("Extraction failed:", err);
  }
}
```
