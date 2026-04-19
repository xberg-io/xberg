<?php

declare(strict_types=1);

namespace Kreuzberg\Types;

/**
 * Content for a single page/slide.
 *
 * When page extraction is enabled, documents are split into per-page content
 * with associated tables and images mapped to each page.
 *
 * @property-read int $pageNumber Page number (1-based)
 * @property-read string $content Page text content
 * @property-read array<Table> $tables Tables found on this page
 * @property-read array<ExtractedImage> $images Images found on this page
 * @property-read ?PageHierarchy $hierarchy Hierarchy information for the page
 * @property-read ?bool $isBlank Whether this page is blank
 * @property-read array<LayoutRegion> $layoutRegions Layout regions detected on this page
 */
readonly class PageContent
{
    /**
     * @param array<Table> $tables
     * @param array<ExtractedImage> $images
     * @param array<LayoutRegion> $layoutRegions
     */
    public function __construct(
        public int $pageNumber,
        public string $content,
        public array $tables = [],
        public array $images = [],
        public ?PageHierarchy $hierarchy = null,
        public ?bool $isBlank = null,
        public array $layoutRegions = [],
    ) {
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        /** @var int $pageNumber */
        $pageNumber = $data['page_number'] ?? 0;

        /** @var string $content */
        $content = $data['content'] ?? '';

        /** @var array<array<string, mixed>> $tablesData */
        $tablesData = $data['tables'] ?? [];

        /** @var array<array<string, mixed>> $imagesData */
        $imagesData = $data['images'] ?? [];

        /** @var ?array<string, mixed> $hierarchyData */
        $hierarchyData = isset($data['hierarchy']) && is_array($data['hierarchy']) ? $data['hierarchy'] : null;

        /** @var ?bool $isBlank */
        $isBlank = isset($data['is_blank']) && is_bool($data['is_blank']) ? $data['is_blank'] : null;

        /** @var array<array<string, mixed>> $layoutRegionsData */
        $layoutRegionsData = isset($data['layout_regions']) && is_array($data['layout_regions'])
            ? $data['layout_regions']
            : [];

        return new self(
            pageNumber: $pageNumber,
            content: $content,
            tables: array_map(
                /** @param array<string, mixed> $table */
                static fn (array $table): Table => Table::fromArray($table),
                $tablesData,
            ),
            images: array_map(
                /** @param array<string, mixed> $image */
                static fn (array $image): ExtractedImage => ExtractedImage::fromArray($image),
                $imagesData,
            ),
            hierarchy: $hierarchyData !== null ? PageHierarchy::fromArray($hierarchyData) : null,
            isBlank: $isBlank,
            layoutRegions: array_map(
                /** @param array<string, mixed> $region */
                static fn (array $region): LayoutRegion => LayoutRegion::fromArray($region),
                $layoutRegionsData,
            ),
        );
    }
}
