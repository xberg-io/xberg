```python title="Python"
from xberg import (
    ExtractionConfig,
    OcrConfig,
    ChunkingConfig,
    ChunkerType,
    ImageExtractionConfig,
    OutputFormat,
    extract_sync,
)

config = ExtractionConfig(
    # OCR: extract text from images, fallback to Tesseract
    ocr=OcrConfig(
        enabled=True,
        backend="tesseract",
        language="eng",
    ),
    # Chunking: semantic markdown chunks of ~800 chars, 100-char overlap
    chunking=ChunkingConfig(
        max_characters=800,
        overlap=100,
        chunker_type=ChunkerType.Markdown,
        prepend_heading_context=True,
    ),
    # Output: Markdown format with document structure preserved
    output_format=OutputFormat.Markdown,
    include_document_structure=True,
    # Images: extract embedded images
    images=ImageExtractionConfig(
        extract_images=True,
    ),
    # Cache extracted results on disk
    use_cache=True,
)

result = extract_sync("report.pdf", config=config)

print(f"Content ({len(result.content)} chars):")
print(result.content[:200])

if result.chunks:
    print(f"\nChunks: {len(result.chunks)}")

print(f"Tables: {len(result.tables)}")

if result.detected_languages:
    print(f"Languages: {result.detected_languages}")

if result.extraction_method:
    print(f"Extraction method: {result.extraction_method}")
```
