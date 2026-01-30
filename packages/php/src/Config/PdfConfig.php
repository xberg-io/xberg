<?php

declare(strict_types=1);

namespace Kreuzberg\Config;

/**
 * PDF extraction configuration.
 */
readonly class PdfConfig
{
    public function __construct(
        /**
         * Extract images from PDF documents.
         *
         * When enabled, images embedded in PDFs are extracted and included
         * in the extraction results. Extracted images can be saved to disk
         * or processed further.
         *
         * @var bool
         * @default false
         */
        public bool $extractImages = false,

        /**
         * PDF passwords for encrypted documents.
         *
         * Provides passwords to try when opening encrypted PDFs. Multiple
         * passwords can be provided and will be tried in order.
         *
         * @var array<string>|null
         * @default null
         */
        public ?array $passwords = null,

        /**
         * Extract PDF metadata.
         *
         * When enabled, PDF metadata such as title, author, subject, keywords,
         * creation date, and modification date are extracted and included
         * in the results.
         *
         * @var bool
         * @default true
         */
        public bool $extractMetadata = true,

        /**
         * Hierarchy configuration for structural extraction.
         *
         * Configures how document structure (headings, sections, etc.)
         * is extracted and represented.
         *
         * @var HierarchyConfig|null
         * @default null
         */
        public ?HierarchyConfig $hierarchy = null,
    ) {
    }

    /**
     * Create configuration from array data.
     *
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        /** @var bool $extractImages */
        $extractImages = $data['extract_images'] ?? false;
        if (!is_bool($extractImages)) {
            /** @var bool $extractImages */
            $extractImages = (bool) $extractImages;
        }

        /** @var array<string>|null $passwords */
        $passwords = $data['passwords'] ?? null;
        if ($passwords !== null && !is_array($passwords)) {
            $passwords = [$passwords];
        }

        /** @var bool $extractMetadata */
        $extractMetadata = $data['extract_metadata'] ?? true;
        if (!is_bool($extractMetadata)) {
            /** @var bool $extractMetadata */
            $extractMetadata = (bool) $extractMetadata;
        }

        $hierarchy = null;
        if (isset($data['hierarchy']) && is_array($data['hierarchy'])) {
            /** @var array<string, mixed> $hierarchyData */
            $hierarchyData = $data['hierarchy'];
            $hierarchy = HierarchyConfig::fromArray($hierarchyData);
        }

        return new self(
            extractImages: $extractImages,
            passwords: $passwords,
            extractMetadata: $extractMetadata,
            hierarchy: $hierarchy,
        );
    }

    /**
     * Create configuration from JSON string.
     */
    public static function fromJson(string $json): self
    {
        $data = json_decode($json, true);
        if (json_last_error() !== JSON_ERROR_NONE) {
            throw new \InvalidArgumentException('Invalid JSON: ' . json_last_error_msg());
        }
        if (!is_array($data)) {
            throw new \InvalidArgumentException('JSON must decode to an object/array');
        }
        /** @var array<string, mixed> $data */
        return self::fromArray($data);
    }

    /**
     * Create configuration from JSON file.
     */
    public static function fromFile(string $path): self
    {
        if (!file_exists($path)) {
            throw new \InvalidArgumentException("File not found: {$path}");
        }
        $contents = file_get_contents($path);
        if ($contents === false) {
            throw new \InvalidArgumentException("Unable to read file: {$path}");
        }
        return self::fromJson($contents);
    }

    /**
     * @return array<string, mixed>
     */
    public function toArray(): array
    {
        return array_filter([
            'extract_images' => $this->extractImages,
            'passwords' => $this->passwords,
            'extract_metadata' => $this->extractMetadata,
            'hierarchy' => $this->hierarchy?->toArray(),
        ], static fn ($value): bool => $value !== null);
    }

    /**
     * Convert configuration to JSON string.
     */
    public function toJson(): string
    {
        $json = json_encode($this->toArray(), JSON_PRETTY_PRINT);
        if ($json === false) {
            throw new \RuntimeException('Failed to encode configuration to JSON');
        }
        return $json;
    }
}
