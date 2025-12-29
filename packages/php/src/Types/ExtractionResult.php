<?php

declare(strict_types=1);

namespace Kreuzberg\Types;

/**
 * Result of document extraction.
 *
 * @property-read string $content Extracted text content
 * @property-read string $mimeType MIME type of the processed document
 * @property-read Metadata $metadata Document metadata
 * @property-read array<Table> $tables Extracted tables
 * @property-read array<string>|null $detectedLanguages Detected language codes (ISO 639-1)
 * @property-read array<Chunk>|null $chunks Text chunks with embeddings and metadata
 * @property-read array<ExtractedImage>|null $images Extracted images (with nested OCR results)
 * @property-read array<PageContent>|null $pages Per-page content when page extraction is enabled
 * @property-read array<mixed, mixed>|null $embeddings Generated embeddings if enabled
 * @property-read array<mixed, mixed>|null $keywords Extracted keywords if enabled
 * @property-read array<mixed, mixed>|null $tesseract Tesseract OCR configuration results if enabled
 */
readonly class ExtractionResult
{
    /**
     * @param array<Table> $tables
     * @param array<string>|null $detectedLanguages
     * @param array<Chunk>|null $chunks
     * @param array<ExtractedImage>|null $images
     * @param array<PageContent>|null $pages
     * @param array<mixed, mixed>|null $embeddings
     * @param array<mixed, mixed>|null $keywords
     * @param array<mixed, mixed>|null $tesseract
     */
    public function __construct(
        public string $content,
        public string $mimeType,
        public Metadata $metadata,
        public array $tables = [],
        public ?array $detectedLanguages = null,
        public ?array $chunks = null,
        public ?array $images = null,
        public ?array $pages = null,
        public ?array $embeddings = null,
        public ?array $keywords = null,
        public ?array $tesseract = null,
    ) {
    }

    /**
     * Create ExtractionResult from array returned by extension.
     *
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        /** @var string $content */
        $content = $data['content'] ?? '';

        /** @var string $mimeType */
        $mimeType = $data['mime_type'] ?? 'application/octet-stream';

        /** @var array<string, mixed> $metadataData */
        $metadataData = $data['metadata'] ?? [];

        /** @var array<array<string, mixed>> $tablesData */
        $tablesData = $data['tables'] ?? [];

        /** @var array<string>|null $detectedLanguages */
        $detectedLanguages = $data['detected_languages'] ?? null;

        $chunks = null;
        if (isset($data['chunks'])) {
            /** @var array<array<string, mixed>> $chunksData */
            $chunksData = $data['chunks'];
            $chunks = array_map(
                /** @param array<string, mixed> $chunk */
                static fn (array $chunk): Chunk => Chunk::fromArray($chunk),
                $chunksData,
            );
        }

        $images = null;
        if (isset($data['images'])) {
            /** @var array<array<string, mixed>> $imagesData */
            $imagesData = $data['images'];
            $images = array_map(
                /** @param array<string, mixed> $image */
                static fn (array $image): ExtractedImage => ExtractedImage::fromArray($image),
                $imagesData,
            );
        }

        $pages = null;
        if (isset($data['pages'])) {
            /** @var array<array<string, mixed>> $pagesData */
            $pagesData = $data['pages'];
            $pages = array_map(
                /** @param array<string, mixed> $page */
                static fn (array $page): PageContent => PageContent::fromArray($page),
                $pagesData,
            );
        }

        $embeddings = $data['embeddings'] ?? null;
        if (!is_array($embeddings)) {
            $embeddings = null;
        }

        $keywords = $data['keywords'] ?? null;
        if (!is_array($keywords)) {
            $keywords = null;
        }

        $tesseract = $data['tesseract'] ?? null;
        if (!is_array($tesseract)) {
            $tesseract = null;
        }

        return new self(
            content: $content,
            mimeType: $mimeType,
            metadata: Metadata::fromArray($metadataData),
            tables: array_map(
                /** @param array<string, mixed> $table */
                static fn (array $table): Table => Table::fromArray($table),
                $tablesData,
            ),
            detectedLanguages: $detectedLanguages,
            chunks: $chunks,
            images: $images,
            pages: $pages,
            embeddings: $embeddings,
            keywords: $keywords,
            tesseract: $tesseract,
        );
    }
}
