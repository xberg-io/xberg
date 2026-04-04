<?php

declare(strict_types=1);

/**
 * Type stubs for Kreuzberg PHP extension functions.
 *
 * These functions are provided by the native Rust extension (kreuzberg.so/.dll).
 * This file provides type hints for IDEs and static analyzers.
 *
 * DO NOT include this file in your application - these functions are automatically
 * available when the extension is loaded.
 *
 * @internal
 */

/**
 * Extract content from a file (native extension function).
 *
 * @param string $filePath Path to the file
 * @param string|null $mimeType Optional MIME type hint
 * @param string|null $config JSON-encoded extraction configuration
 * @return \Kreuzberg\Types\ExtractionResult Extraction result
 * @throws \Exception If extraction fails
 */
function kreuzberg_extract_file(string $filePath, ?string $mimeType, ?string $config): \Kreuzberg\Types\ExtractionResult
{
}

/**
 * Extract content from bytes (native extension function).
 *
 * @param string $data File content as bytes
 * @param string $mimeType MIME type of the data
 * @param string|null $config JSON-encoded extraction configuration
 * @return \Kreuzberg\Types\ExtractionResult Extraction result
 * @throws \Exception If extraction fails
 */
function kreuzberg_extract_bytes(string $data, string $mimeType, ?string $config): \Kreuzberg\Types\ExtractionResult
{
}

/**
 * Extract content from multiple files in parallel (native extension function).
 *
 * @param array<string> $paths List of file paths
 * @param string|null $config JSON-encoded extraction configuration
 * @return array<\Kreuzberg\Types\ExtractionResult> List of extraction results
 * @throws \Exception If extraction fails
 */
function kreuzberg_batch_extract_files(array $paths, ?string $config): array
{
}

/**
 * Extract content from multiple byte arrays in parallel (native extension function).
 *
 * @param array<string> $dataList List of file contents as bytes
 * @param array<string> $mimeTypes List of MIME types
 * @param string|null $config JSON-encoded extraction configuration
 * @return array<\Kreuzberg\Types\ExtractionResult> List of extraction results
 * @throws \Exception If extraction fails
 */
function kreuzberg_batch_extract_bytes(array $dataList, array $mimeTypes, ?string $config): array
{
}

/**
 * Detect MIME type from file bytes (native extension function).
 *
 * @param string $data File content as bytes
 * @return string Detected MIME type
 */
function kreuzberg_detect_mime_type(string $data): string
{
}

/**
 * Detect MIME type from file path (native extension function).
 *
 * @param string $path Path to the file
 * @return string Detected MIME type
 */
function kreuzberg_detect_mime_type_from_path(string $path): string
{
}

/**
 * Register a custom document extractor (native extension function).
 *
 * @param string $mimeType MIME type to handle
 * @param callable $extractor Extractor callback
 * @return void
 */
function kreuzberg_register_extractor(string $mimeType, callable $extractor): void
{
}

/**
 * Unregister a custom document extractor (native extension function).
 *
 * @param string $mimeType MIME type to unregister
 * @return void
 */
function kreuzberg_unregister_extractor(string $mimeType): void
{
}

/**
 * List all registered extractors (native extension function).
 *
 * @return array<string> List of registered MIME types
 */
function kreuzberg_list_extractors(): array
{
}

/**
 * Clear all registered extractors (native extension function).
 *
 * @return void
 */
function kreuzberg_clear_extractors(): void
{
}

/**
 * Test a plugin for compatibility (native extension function).
 *
 * @param string $pluginPath Path to the plugin
 * @return bool Whether the plugin is compatible
 */
function kreuzberg_test_plugin(string $pluginPath): bool
{
}

/**
 * Register a custom OCR backend (native extension function).
 *
 * @param string $name Backend name
 * @param callable $backend Backend callback
 * @return void
 */
function kreuzberg_register_ocr_backend(string $name, callable $backend): void
{
}

/**
 * Unregister a custom OCR backend (native extension function).
 *
 * @param string $name Backend name
 * @return void
 */
function kreuzberg_unregister_ocr_backend(string $name): void
{
}

/**
 * List all registered OCR backends (native extension function).
 *
 * @return array<string> List of registered backend names
 */
function kreuzberg_list_ocr_backends(): array
{
}

/**
 * Register a custom post-processor (native extension function).
 *
 * @param string $name Processor name
 * @param callable $processor Processor callback
 * @return void
 */
function kreuzberg_register_post_processor(string $name, callable $processor): void
{
}

/**
 * Unregister a custom post-processor (native extension function).
 *
 * @param string $name Processor name
 * @return void
 */
function kreuzberg_unregister_post_processor(string $name): void
{
}

/**
 * List all registered post-processors (native extension function).
 *
 * @return array<string> List of registered processor names
 */
function kreuzberg_list_post_processors(): array
{
}

/**
 * Clear all registered post-processors (native extension function).
 *
 * @return void
 */
function kreuzberg_clear_post_processors(): void
{
}

/**
 * Register a custom validator (native extension function).
 *
 * @param string $name Validator name
 * @param callable $validator Validator callback
 * @return void
 */
function kreuzberg_register_validator(string $name, callable $validator): void
{
}

/**
 * Unregister a custom validator (native extension function).
 *
 * @param string $name Validator name
 * @return void
 */
function kreuzberg_unregister_validator(string $name): void
{
}

/**
 * List all registered validators (native extension function).
 *
 * @return array<string> List of registered validator names
 */
function kreuzberg_list_validators(): array
{
}

/**
 * Clear all registered validators (native extension function).
 *
 * @return void
 */
function kreuzberg_clear_validators(): void
{
}

/**
 * Extract content from a file asynchronously (native extension function).
 *
 * @param string $filePath Path to the file
 * @param string|null $mimeType Optional MIME type hint
 * @param string|null $config JSON-encoded extraction configuration
 * @return \Kreuzberg\Types\DeferredResult Deferred result
 * @throws \Exception If config parsing fails
 */
function kreuzberg_extract_file_async(string $filePath, ?string $mimeType = null, ?string $config = null): \Kreuzberg\Types\DeferredResult
{
}

/**
 * Extract content from bytes asynchronously (native extension function).
 *
 * @param string $data File content as bytes
 * @param string $mimeType MIME type of the data
 * @param string|null $config JSON-encoded extraction configuration
 * @return \Kreuzberg\Types\DeferredResult Deferred result
 * @throws \Exception If config parsing fails
 */
function kreuzberg_extract_bytes_async(string $data, string $mimeType, ?string $config = null): \Kreuzberg\Types\DeferredResult
{
}

/**
 * Extract content from multiple files asynchronously (native extension function).
 *
 * @param array<string> $paths List of file paths
 * @param string|null $config JSON-encoded extraction configuration
 * @return \Kreuzberg\Types\DeferredResult Deferred result (use getResults() for batch)
 * @throws \Exception If config parsing fails
 */
function kreuzberg_batch_extract_files_async(array $paths, ?string $config = null): \Kreuzberg\Types\DeferredResult
{
}

/**
 * Extract content from multiple byte arrays asynchronously (native extension function).
 *
 * @param array<string> $dataList List of file contents as bytes
 * @param array<string> $mimeTypes List of MIME types
 * @param string|null $config JSON-encoded extraction configuration
 * @return \Kreuzberg\Types\DeferredResult Deferred result (use getResults() for batch)
 * @throws \Exception If config parsing fails
 */
function kreuzberg_batch_extract_bytes_async(array $dataList, array $mimeTypes, ?string $config = null): \Kreuzberg\Types\DeferredResult
{
}

/**
 * Render a single PDF page to PNG bytes.
 *
 * @param string $filePath Path to the PDF file
 * @param int $pageIndex Zero-based page index
 * @param int|null $dpi Rendering DPI (default: 150)
 * @return string PNG image data
 * @throws \Exception If rendering fails
 */
function kreuzberg_render_pdf_page(string $filePath, int $pageIndex, ?int $dpi = null): string
{
}

/**
 * Create a new PDF page iterator (native extension function).
 *
 * @param string $filePath Path to the PDF file
 * @param int $dpi Rendering resolution
 * @return resource Opaque iterator handle
 * @throws \Exception If creation fails
 */
function kreuzberg_pdf_page_iterator_new(string $filePath, int $dpi): mixed
{
}

/**
 * Advance the PDF page iterator and return the next page as PNG bytes.
 *
 * @param resource $handle Iterator handle from kreuzberg_pdf_page_iterator_new
 * @return string|null PNG-encoded bytes, or null when exhausted
 * @throws \Exception On rendering error
 */
function kreuzberg_pdf_page_iterator_next(mixed $handle): ?string
{
}

/**
 * Free a PDF page iterator handle.
 *
 * @param resource $handle Iterator handle
 * @return void
 */
function kreuzberg_pdf_page_iterator_free(mixed $handle): void
{
}

/**
 * Generate text embeddings (native extension function).
 *
 * @param array<string> $texts List of strings to embed
 * @param string|null $config JSON-encoded embedding configuration
 * @return array<array<float>> List of embedding vectors
 * @throws \Exception If embedding fails
 */
function kreuzberg_embed(array $texts, ?string $config): array
{
}

/**
 * Generate text embeddings asynchronously (native extension function).
 *
 * @param array<string> $texts List of strings to embed
 * @param string|null $config JSON-encoded embedding configuration
 * @return \Kreuzberg\Types\DeferredResult Deferred result
 * @throws \Exception If embedding fails
 */
function kreuzberg_embed_async(array $texts, ?string $config): \Kreuzberg\Types\DeferredResult
{
}
