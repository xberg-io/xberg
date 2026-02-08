# Advanced Features Reference

Kreuzberg provides powerful advanced features for customization, semantic processing, and integration with external systems.

## Plugin System

The plugin system allows you to extend Kreuzberg's extraction pipeline with custom post-processors, validators, and OCR backends. Plugins run within the extraction pipeline and have direct access to extraction results.

### Custom Post-Processors

Post-processors enrich extraction results after document parsing. They run non-destructively—if a post-processor fails, the extraction succeeds anyway (errors are logged).

=== "Python"

    ```python
    from kreuzberg import register_post_processor, ExtractionResult

    class MetadataEnricher:
        def name(self) -> str:
            return "metadata_enricher"

        def process(self, result: ExtractionResult) -> ExtractionResult:
            result.metadata["processed_by"] = "metadata_enricher"
            result.metadata["char_count"] = len(result.content)
            return result

        def processing_stage(self) -> str:
            # "early", "middle", or "late"
            return "middle"

        def initialize(self) -> None:
            print("Initializing metadata enricher")

        def shutdown(self) -> None:
            print("Shutting down metadata enricher")

    register_post_processor(MetadataEnricher())

    # Now use extraction with the registered processor
    from kreuzberg import extract_file_sync
    result = extract_file_sync("document.pdf")
    print(result.metadata["char_count"])
    ```

=== "TypeScript"

    ```typescript
    import { registerPostProcessor, ExtractionResult } from '@kreuzberg/node';

    const enricher = {
        name(): string {
            return "metadata_enricher";
        },

        async process(result: ExtractionResult): Promise<ExtractionResult> {
            result.metadata.processed_by = "metadata_enricher";
            result.metadata.char_count = result.content.length;
            return result;
        },

        processingStage?(): "early" | "middle" | "late" {
            return "middle";
        },

        async initialize?(): Promise<void> {
            console.log("Initializing metadata enricher");
        },

        async shutdown?(): Promise<void> {
            console.log("Shutting down metadata enricher");
        }
    };

    registerPostProcessor(enricher);

    // Now use extraction with the registered processor
    const result = await extractFile("document.pdf");
    console.log(result.metadata.char_count);
    ```

### Custom Validators

Validators perform quality checks on extraction results. Unlike post-processors, validator failures cause the entire extraction to fail. Use validators to enforce quality standards.

=== "Python"

    ```python
    from kreuzberg import register_validator, ExtractionResult, ValidationError

    class MinimumContentValidator:
        def name(self) -> str:
            return "min_content_validator"

        def validate(self, result: ExtractionResult) -> None:
            if len(result.content) < 100:
                raise ValidationError("Extracted content too short (< 100 chars)")

        def priority(self) -> int:
            # Higher priority runs first (0-1000, default 50)
            return 100

        def should_validate(self, result: ExtractionResult) -> bool:
            # Only validate PDFs
            return "pdf" in result.mime_type.lower()

        def initialize(self) -> None:
            pass

        def shutdown(self) -> None:
            pass

    register_validator(MinimumContentValidator())

    # Extraction will fail if content < 100 chars
    result = extract_file_sync("document.pdf")
    ```

=== "TypeScript"

    ```typescript
    import { registerValidator, ExtractionResult } from '@kreuzberg/node';

    const validator = {
        name(): string {
            return "min_content_validator";
        },

        async validate(result: ExtractionResult): Promise<void> {
            if (result.content.length < 100) {
                throw new Error("Extracted content too short (< 100 chars)");
            }
        },

        priority?(): number {
            return 100;
        },

        shouldValidate?(result: ExtractionResult): boolean {
            return result.mimeType.toLowerCase().includes("pdf");
        },

        async initialize?(): Promise<void> {},

        async shutdown?(): Promise<void> {}
    };

    registerValidator(validator);

    // Extraction will fail if content < 100 chars
    const result = await extractFile("document.pdf");
    ```

### Custom OCR Backends

Implement custom OCR engines by registering an OCR backend. This allows integration with proprietary or specialized OCR solutions.

=== "Python"

    ```python
    from kreuzberg import register_ocr_backend

    class CustomOcrBackend:
        def name(self) -> str:
            return "custom_ocr"

        def supported_languages(self) -> list[str]:
            return ["eng", "deu", "fra", "spa"]

        def process_image(self, image_bytes: bytes, language: str) -> dict:
            # image_bytes: raw image data
            # language: ISO 639-3 code (e.g., "eng", "deu")

            # Call your OCR engine here
            # text = my_ocr_engine.recognize(image_bytes, language)

            return {
                "content": "Extracted text from image",
                "metadata": {"confidence": 0.95, "language": language},
                "tables": []
            }

        def process_file(self, path: str, language: str) -> dict:
            # Optional: custom file processing
            # Called when extracting OCR from a file path
            with open(path, "rb") as f:
                image_bytes = f.read()
            return self.process_image(image_bytes, language)

        def initialize(self) -> None:
            # Load models, initialize engine
            pass

        def shutdown(self) -> None:
            # Clean up resources
            pass

        def version(self) -> str:
            return "1.0.0"

    register_ocr_backend(CustomOcrBackend())

    # Use in extraction config
    from kreuzberg import ExtractionConfig, OcrConfig, extract_file_sync

    config = ExtractionConfig(
        ocr=OcrConfig(backend="custom_ocr", language="eng")
    )
    result = extract_file_sync("scanned.pdf", config=config)
    ```

=== "TypeScript"

    ```typescript
    import { registerOcrBackend, ExtractionConfig, extractFile } from '@kreuzberg/node';

    const backend = {
        name(): string {
            return "custom_ocr";
        },

        supportedLanguages(): string[] {
            return ["eng", "deu", "fra", "spa"];
        },

        async processImage(
            imageBytes: Uint8Array | string,
            language: string
        ): Promise<{
            content: string;
            mime_type: string;
            metadata: Record<string, unknown>;
            tables: unknown[];
        }> {
            const buffer = typeof imageBytes === "string"
                ? Buffer.from(imageBytes, "base64")
                : Buffer.from(imageBytes);

            // Call your OCR engine
            // const text = await myOcrEngine.recognize(buffer, language);

            return {
                content: "Extracted text from image",
                mime_type: "text/plain",
                metadata: { confidence: 0.95, language },
                tables: []
            };
        },

        async initialize?(): Promise<void> {
            // Load models, initialize engine
        },

        async shutdown?(): Promise<void> {
            // Clean up resources
        }
    };

    registerOcrBackend(backend);

    // Use in extraction config
    const config: ExtractionConfig = {
        ocr: { backend: "custom_ocr", language: "eng" }
    };
    const result = await extractFile("scanned.pdf", null, config);
    ```

## Embeddings

Generate vector embeddings for text chunks using ONNX-based models. Embeddings enable semantic search, clustering, and similarity operations on extracted content.

**Requirements:** ONNX Runtime 1.22.x or later

=== "Python"

    ```python
    from kreuzberg import (
        ExtractionConfig, ChunkingConfig, EmbeddingConfig,
        EmbeddingModelType, list_embedding_presets,
        get_embedding_preset, extract_file_sync
    )

    # List available embedding presets
    presets = list_embedding_presets()
    print(f"Available presets: {presets}")  # ['balanced', 'compact', 'large']

    # Get details about a preset
    preset_info = get_embedding_preset("balanced")
    print(f"Model: {preset_info.model_name}")
    print(f"Dimensions: {preset_info.dimensions}")
    print(f"Recommended chunk size: {preset_info.chunk_size}")

    # Method 1: Use preset (recommended)
    config = ExtractionConfig(
        chunking=ChunkingConfig(
            max_chars=512,
            max_overlap=100,
            embedding=EmbeddingConfig(
                model=EmbeddingModelType.preset("balanced"),
                normalize=True,
                batch_size=32
            )
        )
    )

    # Method 2: Use specific fastembed model
    config = ExtractionConfig(
        chunking=ChunkingConfig(
            embedding=EmbeddingConfig(
                model=EmbeddingModelType.fastembed(
                    model="BAAI/bge-small-en-v1.5",
                    dimensions=384
                ),
                normalize=True
            )
        )
    )

    # Method 3: Use custom ONNX model from HuggingFace
    config = ExtractionConfig(
        chunking=ChunkingConfig(
            embedding=EmbeddingConfig(
                model=EmbeddingModelType.custom(
                    model_id="sentence-transformers/all-MiniLM-L6-v2",
                    dimensions=384
                ),
                cache_dir="/path/to/model/cache"
            )
        )
    )

    result = extract_file_sync("document.pdf", config=config)

    # Access embeddings in chunks
    for chunk in result.chunks:
        embedding = chunk.embedding  # list[float] or None
        print(f"Chunk: {chunk.content[:50]}...")
        print(f"Embedding dimensions: {len(embedding) if embedding else 0}")
    ```

=== "TypeScript"

    ```typescript
    import {
        ExtractionConfig, ChunkingConfig,
        listEmbeddingPresets, getEmbeddingPreset,
        extractFile
    } from '@kreuzberg/node';

    // List available embedding presets
    const presets = listEmbeddingPresets();
    console.log(`Available presets: ${presets}`);  // ['balanced', 'compact', 'large']

    // Get details about a preset
    const preset = getEmbeddingPreset("balanced");
    console.log(`Model: ${preset.modelName}`);
    console.log(`Dimensions: ${preset.dimensions}`);
    console.log(`Recommended chunk size: ${preset.chunkSize}`);

    // Method 1: Use preset (recommended)
    const config: ExtractionConfig = {
        chunking: {
            maxChars: 512,
            maxOverlap: 100,
            embedding: {
                model: { type: 'preset', name: 'balanced' },
                normalize: true,
                batchSize: 32
            }
        }
    };

    // Method 2: Use specific fastembed model
    const config2: ExtractionConfig = {
        chunking: {
            embedding: {
                model: {
                    type: 'fastembed',
                    model: 'BAAI/bge-small-en-v1.5',
                    dimensions: 384
                },
                normalize: true
            }
        }
    };

    // Method 3: Use custom ONNX model
    const config3: ExtractionConfig = {
        chunking: {
            embedding: {
                model: {
                    type: 'custom',
                    modelId: 'sentence-transformers/all-MiniLM-L6-v2',
                    dimensions: 384
                },
                cacheDir: '/path/to/model/cache'
            }
        }
    };

    const result = await extractFile("document.pdf", null, config);

    // Access embeddings in chunks
    if (result.chunks) {
        for (const chunk of result.chunks) {
            const embedding = chunk.embedding;  // number[] | null
            console.log(`Chunk: ${chunk.content.substring(0, 50)}...`);
            console.log(`Embedding dimensions: ${embedding?.length ?? 0}`);
        }
    }
    ```

## Keyword Extraction

Extract important keywords and phrases from documents using YAKE (Yet Another Keyword Extractor) or RAKE (Rapid Automatic Keyword Extraction) algorithms.

=== "Python"

    ```python
    from kreuzberg import (
        ExtractionConfig, KeywordConfig, KeywordAlgorithm,
        YakeParams, RakeParams, extract_file_sync
    )

    # YAKE algorithm (unsupervised, good for general use)
    config = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.Yake,
            max_keywords=15,
            min_score=0.1,
            ngram_range=(1, 3),
            language="en",
            yake_params=YakeParams(window_size=2)
        )
    )

    # RAKE algorithm (co-occurrence based)
    config = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.Rake,
            max_keywords=10,
            min_score=0.0,
            language="en",
            rake_params=RakeParams(
                min_word_length=3,
                max_words_per_phrase=3
            )
        )
    )

    result = extract_file_sync("document.pdf", config=config)

    # Access extracted keywords
    if result.keywords:
        for keyword in result.keywords:
            print(f"Text: {keyword.text}")
            print(f"Score: {keyword.score}")
            print(f"Algorithm: {keyword.algorithm}")
    ```

=== "TypeScript"

    ```typescript
    import {
        ExtractionConfig, KeywordConfig,
        extractFile
    } from '@kreuzberg/node';

    // YAKE algorithm
    const config: ExtractionConfig = {
        keywords: {
            algorithm: "yake",
            maxKeywords: 15,
            minScore: 0.1,
            ngramRange: [1, 3],
            language: "en",
            yakeParams: {
                windowSize: 2
            }
        }
    };

    // RAKE algorithm
    const config2: ExtractionConfig = {
        keywords: {
            algorithm: "rake",
            maxKeywords: 10,
            minScore: 0.0,
            language: "en",
            rakeParams: {
                minWordLength: 3,
                maxWordsPerPhrase: 3
            }
        }
    };

    const result = await extractFile("document.pdf", null, config);

    // Access extracted keywords
    if (result.keywords) {
        for (const keyword of result.keywords) {
            console.log(`Text: ${keyword.text}`);
            console.log(`Score: ${keyword.score}`);
            console.log(`Algorithm: ${keyword.algorithm}`);
        }
    }
    ```

## Language Detection

Automatically detect the language(s) in documents using ISO 639-1 language codes.

=== "Python"

    ```python
    from kreuzberg import (
        ExtractionConfig, LanguageDetectionConfig,
        extract_file_sync
    )

    # Enable language detection
    config = ExtractionConfig(
        language_detection=LanguageDetectionConfig(
            enabled=True,
            min_confidence=0.8,
            detect_multiple=False
        )
    )

    result = extract_file_sync("multilingual.pdf", config=config)

    # Access detected languages
    if result.detected_languages:
        for lang_code in result.detected_languages:
            print(f"Detected language: {lang_code}")  # e.g., "en", "de", "fr"
    ```

=== "TypeScript"

    ```typescript
    import {
        ExtractionConfig, LanguageDetectionConfig,
        extractFile
    } from '@kreuzberg/node';

    const config: ExtractionConfig = {
        languageDetection: {
            enabled: true,
            minConfidence: 0.8,
            detectMultiple: false
        }
    };

    const result = await extractFile("multilingual.pdf", null, config);

    // Access detected languages
    if (result.detectedLanguages) {
        for (const langCode of result.detectedLanguages) {
            console.log(`Detected language: ${langCode}`);  // e.g., "en", "de", "fr"
        }
    }
    ```

## Token Reduction

Reduce the number of tokens in extracted content for cost optimization when working with LLM APIs. Higher modes are more aggressive but may lose more information.

=== "Python"

    ```python
    from kreuzberg import (
        ExtractionConfig, TokenReductionConfig,
        extract_file_sync
    )

    # Light token reduction
    config = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            mode="light",
            preserve_important_words=True
        )
    )

    # Moderate reduction
    config = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            mode="moderate",
            preserve_important_words=True
        )
    )

    # Aggressive reduction
    config = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            mode="aggressive",
            preserve_important_words=True
        )
    )

    # Maximum reduction
    config = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            mode="maximum",
            preserve_important_words=True
        )
    )

    result = extract_file_sync("document.pdf", config=config)
    print(f"Reduced content length: {len(result.content)}")
    ```

=== "TypeScript"

    ```typescript
    import {
        ExtractionConfig, TokenReductionConfig,
        extractFile
    } from '@kreuzberg/node';

    const config: ExtractionConfig = {
        tokenReduction: {
            mode: "moderate",
            preserveImportantWords: true
        }
    };

    const result = await extractFile("document.pdf", null, config);
    console.log(`Reduced content length: ${result.content.length}`);
    ```

**Token Reduction Modes:**
- `off`: No reduction (default)
- `light`: Remove extra whitespace and redundant punctuation
- `moderate`: Also remove common filler words and some formatting
- `aggressive`: Also remove longer stopwords and collapse similar phrases
- `maximum`: Maximum reduction while preserving semantic content

## Page Extraction

Extract and track per-page content separately. Useful for multi-page documents where you need page-level granularity.

=== "Python"

    ```python
    from kreuzberg import (
        ExtractionConfig, PageConfig,
        extract_file_sync
    )

    config = ExtractionConfig(
        pages=PageConfig(
            extract_pages=True,
            insert_page_markers=True,
            marker_format="\n\n<!-- PAGE {page_num} -->\n\n"
        )
    )

    result = extract_file_sync("multi_page.pdf", config=config)

    # Access per-page content
    if result.pages:
        for page in result.pages:
            print(f"Page {page.page_number}:")
            print(f"Content: {page.content[:100]}...")
            print(f"Tables: {len(page.tables)}")
            print(f"Images: {len(page.images)}")
    ```

=== "TypeScript"

    ```typescript
    import {
        ExtractionConfig, PageExtractionConfig,
        extractFile
    } from '@kreuzberg/node';

    const config: ExtractionConfig = {
        pages: {
            extractPages: true,
            insertPageMarkers: true,
            markerFormat: "\n\n<!-- PAGE {page_num} -->\n\n"
        }
    };

    const result = await extractFile("multi_page.pdf", null, config);

    // Access per-page content
    if (result.pages) {
        for (const page of result.pages) {
            console.log(`Page ${page.pageNumber}:`);
            console.log(`Content: ${page.content.substring(0, 100)}...`);
            console.log(`Tables: ${page.tables.length}`);
            console.log(`Images: ${page.images.length}`);
        }
    }
    ```

## Element-Based Output

Extract semantic elements instead of unified content. This format is compatible with the Unstructured library and provides structured access to different content types (titles, headings, text, tables, images, etc.).

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, ResultFormat, extract_file_sync

    config = ExtractionConfig(
        result_format="element_based"
    )

    result = extract_file_sync("document.pdf", config=config)

    # Access semantic elements
    if result.elements:
        for element in result.elements:
            print(f"Type: {element.element_type}")  # title, heading, narrative_text, etc.
            print(f"Text: {element.text}")
            if element.metadata.get("page_number"):
                print(f"Page: {element.metadata['page_number']}")
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, extractFile } from '@kreuzberg/node';

    const config: ExtractionConfig = {
        resultFormat: "element_based"
    };

    const result = await extractFile("document.pdf", null, config);

    // Access semantic elements
    if (result.elements) {
        for (const element of result.elements) {
            console.log(`Type: ${element.elementType}`);
            console.log(`Text: ${element.text}`);
            if (element.metadata.pageNumber) {
                console.log(`Page: ${element.metadata.pageNumber}`);
            }
        }
    }
    ```

**Element Types:**
- `title`: Document or section title
- `heading`: Section headings
- `narrative_text`: Regular paragraph text
- `list_item`: Items in bullet/numbered lists
- `table`: Table structures
- `image`: Images or figures
- `page_break`: Page boundaries
- `code_block`: Code snippets
- `block_quote`: Quoted text
- `footer`: Footer content
- `header`: Header content

## Djot Content

Output extracted content in Djot markup format (a lighter alternative to Markdown with enhanced structure).

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, OutputFormat, extract_file_sync

    config = ExtractionConfig(
        output_format="djot"
    )

    result = extract_file_sync("document.pdf", config=config)
    print(result.content)  # Djot-formatted content

    # Access structured Djot content
    if result.djot_content:
        print(f"Plain text: {result.djot_content['plain_text']}")
        print(f"Blocks: {result.djot_content['blocks']}")
        print(f"Links: {result.djot_content['links']}")
        print(f"Images: {result.djot_content['images']}")
        print(f"Footnotes: {result.djot_content['footnotes']}")
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, extractFile } from '@kreuzberg/node';

    const config: ExtractionConfig = {
        outputFormat: "djot"
    };

    const result = await extractFile("document.pdf", null, config);
    console.log(result.content);  // Djot-formatted content

    // Access structured Djot content (if available)
    if (result.djotContent) {
        console.log(`Plain text: ${result.djotContent.plain_text}`);
        console.log(`Blocks: ${result.djotContent.blocks}`);
        console.log(`Links: ${result.djotContent.links}`);
        console.log(`Images: ${result.djotContent.images}`);
        console.log(`Footnotes: ${result.djotContent.footnotes}`);
    }
    ```

## API Server

Run Kreuzberg as an HTTP API server for integration with external services.

```bash
# Start server on default port 8000
kreuzberg serve

# Custom host and port
kreuzberg serve --host 0.0.0.0 --port 9000

# Enable CORS and other options
kreuzberg serve --host localhost --port 8000
```

**API Endpoints:**

- `POST /extract` - Extract from uploaded file
- `POST /batch` - Batch extraction
- `POST /detect` - Detect MIME type

**Example:**

```bash
curl -X POST -F "file=@document.pdf" http://localhost:8000/extract
```

## MCP Server

Run Kreuzberg as a Model Context Protocol server for integration with Claude and other AI models.

```bash
# Start MCP server with stdio transport
kreuzberg mcp --transport stdio

# Start MCP server with HTTP transport
kreuzberg mcp --transport http --host 127.0.0.1 --port 8001
```

The MCP server exposes extraction functions to AI models, allowing them to process documents directly.

## Security Limits

Set resource limits to prevent abuse and control memory/file size consumption.

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, extract_file_sync

    config = ExtractionConfig(
        security_limits={
            "max_file_size": 100_000_000,      # 100 MB
            "max_archive_files": 1000,
            "max_text_length": 10_000_000,     # 10 MB of text
            "max_pages": 10000,
            "max_concurrent_extractions": 4
        }
    )

    result = extract_file_sync("document.pdf", config=config)
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, extractFile } from '@kreuzberg/node';

    const config: ExtractionConfig = {
        securityLimits: {
            max_file_size: 100_000_000,        // 100 MB
            max_archive_files: 1000,
            max_text_length: 10_000_000,       // 10 MB of text
            max_pages: 10000,
            max_concurrent_extractions: 4
        }
    };

    const result = await extractFile("document.pdf", null, config);
    ```

**Common Limits:**
- `max_file_size`: Maximum input file size in bytes
- `max_archive_files`: Maximum files in archives (zip, tar, etc.)
- `max_text_length`: Maximum extracted text length
- `max_pages`: Maximum number of pages to process
- `max_concurrent_extractions`: Maximum concurrent extraction operations

## Caching

Extraction results are cached by default to improve performance on repeated extractions of identical documents. Control caching behavior through configuration.

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, extract_file_sync

    # Enable caching (default)
    config = ExtractionConfig(use_cache=True)
    result = extract_file_sync("document.pdf", config=config)

    # Disable caching for a specific extraction
    config = ExtractionConfig(use_cache=False)
    result = extract_file_sync("document.pdf", config=config)
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, extractFile } from '@kreuzberg/node';

    // Enable caching (default)
    const config: ExtractionConfig = { useCache: true };
    const result = await extractFile("document.pdf", null, config);

    // Disable caching
    const config2: ExtractionConfig = { useCache: false };
    const result2 = await extractFile("document.pdf", null, config2);
    ```

**CLI Cache Management:**

```bash
# View cache statistics
kreuzberg cache stats

# Clear all cached results
kreuzberg cache clear
```

Caching is transparent and automatic—same input produces cached output instantly on subsequent extractions.
