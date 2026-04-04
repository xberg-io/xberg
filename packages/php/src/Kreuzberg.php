<?php

declare(strict_types=1);

namespace Kreuzberg;

use Kreuzberg\Config\EmbeddingConfig;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Types\DeferredResult;
use Kreuzberg\Types\ExtractionResult;

/**
 * Main Kreuzberg API class for document extraction.
 *
 * Provides high-performance document intelligence powered by a Rust core.
 * Extract text, metadata, and structured data from PDFs, Office documents,
 * images, and 75+ file formats.
 *
 * @example
 * ```php
 * use Kreuzberg\Kreuzberg;
 * use Kreuzberg\Config\ExtractionConfig;
 * use Kreuzberg\Config\OcrConfig;
 *
 * $kreuzberg = new Kreuzberg();
 * $result = $kreuzberg->extractFile('document.pdf');
 * echo $result->content;
 *
 * // With configuration
 * $config = new ExtractionConfig(
 *     ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
 * );
 * $kreuzberg = new Kreuzberg($config);
 * $result = $kreuzberg->extractFile('scanned.pdf');
 * ```
 */
final readonly class Kreuzberg
{
    public const VERSION = '4.6.3';

    public function __construct(
        private ?ExtractionConfig $defaultConfig = null,
        private ?EmbeddingConfig $defaultEmbeddingConfig = null,
    ) {
    }

    /**
     * Extract content from a file.
     *
     * @param string $filePath Path to the file to extract
     * @param string|null $mimeType Optional MIME type hint (auto-detected if null)
     * @param ExtractionConfig|null $config Extraction configuration (uses constructor config if null)
     * @return ExtractionResult Extraction result with content, metadata, and tables
     * @throws KreuzbergException If extraction fails
     */
    public function extractFile(
        string $filePath,
        ?string $mimeType = null,
        ?ExtractionConfig $config = null,
    ): ExtractionResult {
        $config ??= $this->defaultConfig ?? new ExtractionConfig();

        return extract_file($filePath, $mimeType, $config);
    }

    /**
     * Extract content from bytes.
     *
     * @param string $data File content as bytes
     * @param string $mimeType MIME type of the data (required for format detection)
     * @param ExtractionConfig|null $config Extraction configuration (uses constructor config if null)
     * @return ExtractionResult Extraction result with content, metadata, and tables
     * @throws KreuzbergException If extraction fails
     */
    public function extractBytes(
        string $data,
        string $mimeType,
        ?ExtractionConfig $config = null,
    ): ExtractionResult {
        $config ??= $this->defaultConfig ?? new ExtractionConfig();

        return extract_bytes($data, $mimeType, $config);
    }

    /**
     * Extract content from multiple files in parallel.
     *
     * @param array<string> $paths List of file paths
     * @param ExtractionConfig|null $config Extraction configuration (uses constructor config if null)
     * @return array<ExtractionResult> List of extraction results (one per file)
     * @throws KreuzbergException If extraction fails
     */
    public function batchExtractFiles(
        array $paths,
        ?ExtractionConfig $config = null,
    ): array {
        $config ??= $this->defaultConfig ?? new ExtractionConfig();

        return batch_extract_files($paths, $config);
    }

    /**
     * Extract content from multiple byte arrays in parallel.
     *
     * @param array<string> $dataList List of file contents as bytes
     * @param array<string> $mimeTypes List of MIME types (one per data item)
     * @param ExtractionConfig|null $config Extraction configuration (uses constructor config if null)
     * @return array<ExtractionResult> List of extraction results (one per data item)
     * @throws KreuzbergException If extraction fails
     */
    public function batchExtractBytes(
        array $dataList,
        array $mimeTypes,
        ?ExtractionConfig $config = null,
    ): array {
        $config ??= $this->defaultConfig ?? new ExtractionConfig();

        return batch_extract_bytes($dataList, $mimeTypes, $config);
    }

    /**
     * Extract content from a file asynchronously.
     *
     * Returns a DeferredResult immediately. The extraction runs on a background thread.
     *
     * @param string $filePath Path to the file to extract
     * @param string|null $mimeType Optional MIME type hint (auto-detected if null)
     * @param ExtractionConfig|null $config Extraction configuration (uses constructor config if null)
     * @return DeferredResult Deferred result that can be polled or waited on
     * @throws KreuzbergException If config parsing fails
     */
    public function extractFileAsync(
        string $filePath,
        ?string $mimeType = null,
        ?ExtractionConfig $config = null,
    ): DeferredResult {
        $config ??= $this->defaultConfig ?? new ExtractionConfig();

        return extract_file_async($filePath, $mimeType, $config);
    }

    /**
     * Extract content from bytes asynchronously.
     *
     * @param string $data File content as bytes
     * @param string $mimeType MIME type of the data
     * @param ExtractionConfig|null $config Extraction configuration (uses constructor config if null)
     * @return DeferredResult Deferred result that can be polled or waited on
     * @throws KreuzbergException If config parsing fails
     */
    public function extractBytesAsync(
        string $data,
        string $mimeType,
        ?ExtractionConfig $config = null,
    ): DeferredResult {
        $config ??= $this->defaultConfig ?? new ExtractionConfig();

        return extract_bytes_async($data, $mimeType, $config);
    }

    /**
     * Extract content from multiple files asynchronously.
     *
     * @param array<string> $paths List of file paths
     * @param ExtractionConfig|null $config Extraction configuration (uses constructor config if null)
     * @return DeferredResult Deferred result (use getResults() for batch)
     * @throws KreuzbergException If config parsing fails
     */
    public function batchExtractFilesAsync(
        array $paths,
        ?ExtractionConfig $config = null,
    ): DeferredResult {
        $config ??= $this->defaultConfig ?? new ExtractionConfig();

        return batch_extract_files_async($paths, $config);
    }

    /**
     * Extract content from multiple byte arrays asynchronously.
     *
     * @param array<string> $dataList List of file contents as bytes
     * @param array<string> $mimeTypes List of MIME types (one per data item)
     * @param ExtractionConfig|null $config Extraction configuration (uses constructor config if null)
     * @return DeferredResult Deferred result (use getResults() for batch)
     * @throws KreuzbergException If config parsing fails
     */
    public function batchExtractBytesAsync(
        array $dataList,
        array $mimeTypes,
        ?ExtractionConfig $config = null,
    ): DeferredResult {
        $config ??= $this->defaultConfig ?? new ExtractionConfig();

        return batch_extract_bytes_async($dataList, $mimeTypes, $config);
    }

    /**
     * Generate text embeddings for a list of strings.
     *
     * @param array<string> $texts List of strings to embed
     * @param EmbeddingConfig|null $config Embedding configuration (uses constructor config if null)
     * @return array<array<float>> List of embedding vectors (one per input string)
     * @throws KreuzbergException If generation fails
     */
    public function embed(
        array $texts,
        ?EmbeddingConfig $config = null,
    ): array {
        $config ??= $this->defaultEmbeddingConfig ?? new EmbeddingConfig();

        return embed($texts, $config);
    }

    /**
     * Generate text embeddings asynchronously.
     *
     * @param array<string> $texts List of strings to embed
     * @param EmbeddingConfig|null $config Embedding configuration (uses constructor config if null)
     * @return DeferredResult Deferred result that can be polled or waited on
     * @throws KreuzbergException If generation fails
     */
    public function embedAsync(
        array $texts,
        ?EmbeddingConfig $config = null,
    ): DeferredResult {
        $config ??= $this->defaultEmbeddingConfig ?? new EmbeddingConfig();

        return embed_async($texts, $config);
    }

    /**
     * Extract content from a file (static synchronous method).
     *
     * @param string $filePath Path to the file to extract
     * @param string|null $mimeType Optional MIME type hint (auto-detected if null)
     * @param ExtractionConfig|null $config Extraction configuration (uses defaults if null)
     * @return ExtractionResult Extraction result with content, metadata, and tables
     * @throws KreuzbergException If extraction fails
     */
    public static function extractFileSync(
        string $filePath,
        ?string $mimeType = null,
        ?ExtractionConfig $config = null,
    ): ExtractionResult {
        $config ??= new ExtractionConfig();

        return extract_file($filePath, $mimeType, $config);
    }

    /**
     * Extract content from bytes (static synchronous method).
     *
     * @param string $data File content as bytes
     * @param string $mimeType MIME type of the data (required for format detection)
     * @param ExtractionConfig|null $config Extraction configuration (uses defaults if null)
     * @return ExtractionResult Extraction result with content, metadata, and tables
     * @throws KreuzbergException If extraction fails
     */
    public static function extractBytesSync(
        string $data,
        string $mimeType,
        ?ExtractionConfig $config = null,
    ): ExtractionResult {
        $config ??= new ExtractionConfig();

        return extract_bytes($data, $mimeType, $config);
    }

    /**
     * Extract content from multiple files in parallel (static synchronous method).
     *
     * @param array<string> $paths List of file paths
     * @param ExtractionConfig|null $config Extraction configuration (uses defaults if null)
     * @return array<ExtractionResult> List of extraction results (one per file)
     * @throws KreuzbergException If extraction fails
     */
    public static function batchExtractFilesSync(
        array $paths,
        ?ExtractionConfig $config = null,
    ): array {
        $config ??= new ExtractionConfig();

        return batch_extract_files($paths, $config);
    }

    /**
     * Extract content from multiple byte arrays in parallel (static synchronous method).
     *
     * @param array<string> $dataList List of file contents as bytes
     * @param array<string> $mimeTypes List of MIME types (one per data item)
     * @param ExtractionConfig|null $config Extraction configuration (uses defaults if null)
     * @return array<ExtractionResult> List of extraction results (one per data item)
     * @throws KreuzbergException If extraction fails
     */
    public static function batchExtractBytesSync(
        array $dataList,
        array $mimeTypes,
        ?ExtractionConfig $config = null,
    ): array {
        $config ??= new ExtractionConfig();

        return batch_extract_bytes($dataList, $mimeTypes, $config);
    }

    /**
     * Extract content from a file asynchronously (static method).
     *
     * @param string $filePath Path to the file to extract
     * @param string|null $mimeType Optional MIME type hint (auto-detected if null)
     * @param ExtractionConfig|null $config Extraction configuration (uses defaults if null)
     * @return DeferredResult Deferred result that can be polled or waited on
     * @throws KreuzbergException If config parsing fails
     */
    public static function extractFileAsyncStatic(
        string $filePath,
        ?string $mimeType = null,
        ?ExtractionConfig $config = null,
    ): DeferredResult {
        $config ??= new ExtractionConfig();

        return extract_file_async($filePath, $mimeType, $config);
    }

    /**
     * Extract content from bytes asynchronously (static method).
     *
     * @param string $data File content as bytes
     * @param string $mimeType MIME type of the data
     * @param ExtractionConfig|null $config Extraction configuration (uses defaults if null)
     * @return DeferredResult Deferred result that can be polled or waited on
     * @throws KreuzbergException If config parsing fails
     */
    public static function extractBytesAsyncStatic(
        string $data,
        string $mimeType,
        ?ExtractionConfig $config = null,
    ): DeferredResult {
        $config ??= new ExtractionConfig();

        return extract_bytes_async($data, $mimeType, $config);
    }

    /**
     * Extract content from multiple files asynchronously (static method).
     *
     * @param array<string> $paths List of file paths
     * @param ExtractionConfig|null $config Extraction configuration (uses defaults if null)
     * @return DeferredResult Deferred result (use getResults() for batch)
     * @throws KreuzbergException If config parsing fails
     */
    public static function batchExtractFilesAsyncStatic(
        array $paths,
        ?ExtractionConfig $config = null,
    ): DeferredResult {
        $config ??= new ExtractionConfig();

        return batch_extract_files_async($paths, $config);
    }

    /**
     * Extract content from multiple byte arrays asynchronously (static method).
     *
     * @param array<string> $dataList List of file contents as bytes
     * @param array<string> $mimeTypes List of MIME types (one per data item)
     * @param ExtractionConfig|null $config Extraction configuration (uses defaults if null)
     * @return DeferredResult Deferred result (use getResults() for batch)
     * @throws KreuzbergException If config parsing fails
     */
    public static function batchExtractBytesAsyncStatic(
        array $dataList,
        array $mimeTypes,
        ?ExtractionConfig $config = null,
    ): DeferredResult {
        $config ??= new ExtractionConfig();

        return batch_extract_bytes_async($dataList, $mimeTypes, $config);
    }

    /**
     * Generate text embeddings for a list of strings (static).
     *
     * @param array<string> $texts List of strings to embed
     * @param EmbeddingConfig|null $config Embedding configuration (uses defaults if null)
     * @return array<array<float>> List of embedding vectors (one per input string)
     * @throws KreuzbergException If generation fails
     */
    public static function embedStatic(
        array $texts,
        ?EmbeddingConfig $config = null,
    ): array {
        return embed($texts, $config);
    }

    /**
     * Generate text embeddings asynchronously (static).
     *
     * @param array<string> $texts List of strings to embed
     * @param EmbeddingConfig|null $config Embedding configuration (uses defaults if null)
     * @return DeferredResult Deferred result that can be polled or waited on
     * @throws KreuzbergException If generation fails
     */
    public static function embedAsyncStatic(
        array $texts,
        ?EmbeddingConfig $config = null,
    ): DeferredResult {
        return embed_async($texts, $config);
    }

    /**
     * Detect MIME type from file bytes.
     *
     * @param string $data File content as bytes
     * @return string Detected MIME type (e.g., "application/pdf", "image/png")
     * @throws KreuzbergException If MIME type detection fails
     */
    public static function detectMimeType(string $data): string
    {
        return \Kreuzberg\detect_mime_type($data);
    }

    /**
     * Detect MIME type from file path.
     *
     * @param string $path Path to the file
     * @return string Detected MIME type (e.g., "application/pdf", "text/plain")
     * @throws KreuzbergException If MIME type detection fails
     */
    public static function detectMimeTypeFromPath(string $path): string
    {
        return \Kreuzberg\detect_mime_type_from_path($path);
    }

    /**
     * Get file extensions for a MIME type.
     *
     * @param string $mimeType MIME type (e.g., "application/pdf")
     * @return array<string> List of file extensions (e.g., ["pdf"])
     * @throws KreuzbergException If extensions lookup fails
     */
    public static function getExtensionsForMime(string $mimeType): array
    {
        /** @var array<string> $result */
        $result = \kreuzberg_get_extensions_for_mime($mimeType);
        return $result;
    }

    /**
     * Clear all registered document extractors.
     *
     * @throws KreuzbergException If clear operation fails
     */
    public static function clearDocumentExtractors(): void
    {
        \kreuzberg_clear_extractors();
    }

    /**
     * List all registered document extractors.
     *
     * @return array<string> List of extractor names
     * @throws KreuzbergException If list operation fails
     */
    public static function listDocumentExtractors(): array
    {
        /** @var array<string> $result */
        $result = \kreuzberg_list_extractors();
        return $result;
    }

    /**
     * Unregister a document extractor by name.
     *
     * @param string $name Name of the extractor to unregister
     * @throws KreuzbergException If unregister operation fails
     */
    public static function unregisterDocumentExtractor(string $name): void
    {
        \kreuzberg_unregister_extractor($name);
    }

    /**
     * Clear all registered OCR backends.
     *
     * @throws KreuzbergException If clear operation fails
     */
    public static function clearOcrBackends(): void
    {
        \kreuzberg_clear_ocr_backends();
    }

    /**
     * List all registered OCR backends.
     *
     * @return array<string> List of backend names
     * @throws KreuzbergException If list operation fails
     */
    public static function listOcrBackends(): array
    {
        /** @var array<string> $result */
        $result = \kreuzberg_list_ocr_backends();
        return $result;
    }

    /**
     * Unregister an OCR backend by name.
     *
     * @param string $name Name of the backend to unregister
     * @throws KreuzbergException If unregister operation fails
     */
    public static function unregisterOcrBackend(string $name): void
    {
        \kreuzberg_unregister_ocr_backend($name);
    }

    /**
     * Clear all registered post-processors.
     *
     * @throws KreuzbergException If clear operation fails
     */
    public static function clearPostProcessors(): void
    {
        \kreuzberg_clear_post_processors();
    }

    /**
     * List all registered post-processors.
     *
     * @return array<string> List of post-processor names
     * @throws KreuzbergException If list operation fails
     */
    public static function listPostProcessors(): array
    {
        /** @var array<string> $result */
        $result = \kreuzberg_list_post_processors();
        return $result;
    }

    /**
     * Clear all registered validators.
     *
     * @throws KreuzbergException If clear operation fails
     */
    public static function clearValidators(): void
    {
        \kreuzberg_clear_validators();
    }

    /**
     * List all registered validators.
     *
     * @return array<string> List of validator names
     * @throws KreuzbergException If list operation fails
     */
    public static function listValidators(): array
    {
        /** @var array<string> $result */
        $result = \kreuzberg_list_validators();
        return $result;
    }

    /**
     * Get the library version.
     */
    public static function version(): string
    {
        return self::VERSION;
    }
}
