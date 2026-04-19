use crate::fixtures::{
    Assertions, ExtractionMethod, Fixture, GenerationMode, InputType, PluginAssertions, PluginTestSpec,
    RenderAssertions,
};
use crate::parity::{self, ParityManifest, TypeDef};
use anyhow::{Context, Result};
use camino::Utf8Path;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;

const PHP_HELPERS_TEMPLATE: &str = r#"<?php

declare(strict_types=1);

namespace E2EPhp;

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Types\ExtractionResult;
use PHPUnit\Framework\Assert;

class Helpers
{
    private static ?string $workspaceRoot = null;
    private static ?string $testDocuments = null;

    public static function getWorkspaceRoot(): string
    {
        if (self::$workspaceRoot === null) {
            $dir = __DIR__;
            while (true) {
                if (is_dir($dir . '/test_documents')) {
                    self::$workspaceRoot = $dir;
                    break;
                }
                $parent = dirname($dir);
                if ($parent === $dir) {
                    throw new \RuntimeException('Could not find workspace root (directory containing test_documents/)');
                }
                $dir = $parent;
            }
        }
        return self::$workspaceRoot;
    }

    public static function getTestDocuments(): string
    {
        if (self::$testDocuments === null) {
            self::$testDocuments = self::getWorkspaceRoot() . '/test_documents';
        }
        return self::$testDocuments;
    }

    public static function resolveDocument(string $relative): string
    {
        return self::getTestDocuments() . '/' . $relative;
    }

    public static function buildConfig(?array $config): ?ExtractionConfig
    {
        if ($config === null || empty($config)) {
            return null;
        }

        // Transform embedding model from Rust object format to PHP string format.
        // Fixtures define model as {"type": "preset", "name": "balanced"} but
        // PHP's EmbeddingConfig expects just the preset name string "balanced".
        if (isset($config['chunking']['embedding']['model'])
            && is_array($config['chunking']['embedding']['model'])
        ) {
            $model = $config['chunking']['embedding']['model'];
            if (isset($model['type']) && $model['type'] === 'preset' && isset($model['name'])) {
                $config['chunking']['embedding']['model'] = $model['name'];
            }
        }

        return ExtractionConfig::fromArray($config);
    }

    public static function assertExpectedMime(ExtractionResult $result, array $expected): void
    {
        if (empty($expected)) {
            return;
        }

        $matches = false;
        foreach ($expected as $token) {
            if (str_contains($result->mimeType, $token)) {
                $matches = true;
                break;
            }
        }

        Assert::assertTrue(
            $matches,
            sprintf(
                "Expected MIME '%s' to match one of %s",
                $result->mimeType,
                json_encode($expected)
            )
        );
    }

    public static function assertMinContentLength(ExtractionResult $result, int $minimum): void
    {
        Assert::assertGreaterThanOrEqual(
            $minimum,
            strlen($result->content),
            sprintf("Expected content length >= %d, got %d", $minimum, strlen($result->content))
        );
    }

    public static function assertMaxContentLength(ExtractionResult $result, int $maximum): void
    {
        Assert::assertLessThanOrEqual(
            $maximum,
            strlen($result->content),
            sprintf("Expected content length <= %d, got %d", $maximum, strlen($result->content))
        );
    }

    public static function assertContentContainsAny(ExtractionResult $result, array $snippets): void
    {
        if (empty($snippets)) {
            return;
        }

        $lowered = strtolower($result->content);
        $found = false;
        foreach ($snippets as $snippet) {
            if (str_contains($lowered, strtolower($snippet))) {
                $found = true;
                break;
            }
        }

        Assert::assertTrue(
            $found,
            sprintf(
                "Expected content to contain any of %s. Preview: %s",
                json_encode($snippets),
                json_encode(substr($result->content, 0, 160))
            )
        );
    }

    public static function assertContentContainsAll(ExtractionResult $result, array $snippets): void
    {
        if (empty($snippets)) {
            return;
        }

        $lowered = strtolower($result->content);
        $missing = [];
        foreach ($snippets as $snippet) {
            if (!str_contains($lowered, strtolower($snippet))) {
                $missing[] = $snippet;
            }
        }

        Assert::assertEmpty(
            $missing,
            sprintf(
                "Expected content to contain all snippets %s. Missing %s",
                json_encode($snippets),
                json_encode($missing)
            )
        );
    }

    public static function assertContentContainsNone(ExtractionResult $result, array $snippets): void
    {
        if (empty($snippets)) {
            return;
        }

        $lowered = strtolower($result->content);
        $found = [];
        foreach ($snippets as $snippet) {
            if (str_contains($lowered, strtolower($snippet))) {
                $found[] = $snippet;
            }
        }

        Assert::assertEmpty(
            $found,
            sprintf(
                "Expected content to contain none of %s. Found %s",
                json_encode($snippets),
                json_encode($found)
            )
        );
    }

    public static function assertTableCount(ExtractionResult $result, ?int $minimum, ?int $maximum): void
    {
        $count = count($result->tables ?? []);

        if ($minimum !== null) {
            Assert::assertGreaterThanOrEqual(
                $minimum,
                $count,
                sprintf("Expected at least %d tables, found %d", $minimum, $count)
            );
        }

        if ($maximum !== null) {
            Assert::assertLessThanOrEqual(
                $maximum,
                $count,
                sprintf("Expected at most %d tables, found %d", $maximum, $count)
            );
        }
    }

    public static function assertDetectedLanguages(
        ExtractionResult $result,
        array $expected,
        ?float $minConfidence
    ): void {
        if (empty($expected)) {
            return;
        }

        Assert::assertNotNull($result->detectedLanguages, "Expected detected languages but field is null");

        $missing = [];
        foreach ($expected as $lang) {
            if (!in_array($lang, $result->detectedLanguages, true)) {
                $missing[] = $lang;
            }
        }

        Assert::assertEmpty(
            $missing,
            sprintf("Expected languages %s, missing %s", json_encode($expected), json_encode($missing))
        );

        $metaArr = self::metadataToArray($result->metadata);
        if ($minConfidence !== null && isset($metaArr['confidence'])) {
            $confidence = $metaArr['confidence'];
            Assert::assertGreaterThanOrEqual(
                $minConfidence,
                $confidence,
                sprintf("Expected confidence >= %f, got %f", $minConfidence, $confidence)
            );
        }
    }

    public static function assertChunks(
        $result,
        ?int $minCount = null,
        ?int $maxCount = null,
        ?bool $eachHasContent = null,
        ?bool $eachHasEmbedding = null,
        ?bool $eachHasHeadingContext = null,
        ?bool $eachHasChunkType = null,
        ?bool $contentStartsWithHeading = null
    ): void {
        $chunks = $result->chunks ?? null;
        if ($chunks === null) {
            throw new \Exception("Expected chunks but field is null");
        }

        $count = count($chunks);
        if ($minCount !== null && $count < $minCount) {
            throw new \Exception("Expected at least $minCount chunks, found $count");
        }
        if ($maxCount !== null && $count > $maxCount) {
            throw new \Exception("Expected at most $maxCount chunks, found $count");
        }

        foreach ($chunks as $i => $chunk) {
            if ($eachHasContent && empty($chunk->content)) {
                throw new \Exception("Chunk $i has no content");
            }
            if ($eachHasEmbedding && (empty($chunk->embedding))) {
                throw new \Exception("Chunk $i has no embedding");
            }
            if ($eachHasHeadingContext !== null) {
                $hc = $chunk->metadata->headingContext ?? null;
                if ($eachHasHeadingContext && $hc === null) {
                    throw new \Exception("Chunk $i has no headingContext");
                }
                if (!$eachHasHeadingContext && $hc !== null) {
                    throw new \Exception("Chunk $i should have no headingContext");
                }
            }
            if ($eachHasChunkType === true) {
                $type = $chunk->chunkType ?? $chunk->chunk_type ?? null;
                if ($type === null || $type === "unknown") {
                    throw new \Exception("Chunk $i has no specific chunkType, got " . var_export($type, true));
                }
            }
        }
        if ($contentStartsWithHeading === true) {
            foreach ($chunks as $i => $chunk) {
                if (empty($chunk->metadata->heading_context ?? null)) continue;
                Assert::assertStringStartsWith(
                    '#',
                    $chunk->content ?? '',
                    sprintf("Chunk %d content should start with a heading (#)", $i)
                );
            }
        }
    }

    public static function assertImages(
        ExtractionResult $result,
        ?int $minCount,
        ?int $maxCount,
        ?array $formatsInclude
    ): void {
        $images = $result->images ?? [];
        $count = count($images);

        if ($minCount !== null) {
            Assert::assertGreaterThanOrEqual(
                $minCount,
                $count,
                sprintf("Expected at least %d images, found %d", $minCount, $count)
            );
        }

        if ($maxCount !== null) {
            Assert::assertLessThanOrEqual(
                $maxCount,
                $count,
                sprintf("Expected at most %d images, found %d", $maxCount, $count)
            );
        }

        if ($formatsInclude !== null && !empty($formatsInclude)) {
            $foundFormats = [];
            foreach ($images as $image) {
                if (isset($image->format)) {
                    $foundFormats[] = strtolower($image->format);
                }
            }

            foreach ($formatsInclude as $format) {
                Assert::assertContains(
                    strtolower($format),
                    $foundFormats,
                    sprintf("Expected image format '%s' not found in %s", $format, json_encode($foundFormats))
                );
            }
        }
    }

    public static function assertPages(
        ExtractionResult $result,
        ?int $minCount,
        ?int $exactCount,
        ?bool $hasLayoutRegions,
        ?array $layoutClassesInclude
    ): void {
        $pages = $result->pages ?? [];
        $count = count($pages);

        if ($exactCount !== null) {
            Assert::assertEquals(
                $exactCount,
                $count,
                sprintf("Expected exactly %d pages, found %d", $exactCount, $count)
            );
        }

        if ($minCount !== null) {
            Assert::assertGreaterThanOrEqual(
                $minCount,
                $count,
                sprintf("Expected at least %d pages, found %d", $minCount, $count)
            );
        }

        foreach ($pages as $page) {
            if (property_exists($page, 'isBlank')) {
                Assert::assertTrue(
                    $page->isBlank === null || is_bool($page->isBlank),
                    'isBlank should be null or bool'
                );
            }
        }

        if ($hasLayoutRegions === true) {
            $foundLayoutRegions = false;
            foreach ($pages as $page) {
                if (
                    property_exists($page, 'layoutRegions')
                    && $page->layoutRegions !== null
                    && count($page->layoutRegions) > 0
                ) {
                    $foundLayoutRegions = true;
                    break;
                }
            }
            Assert::assertTrue(
                $foundLayoutRegions,
                'Expected at least one page to have layout_regions populated'
            );
        }

        if ($layoutClassesInclude !== null && count($layoutClassesInclude) > 0) {
            $allClasses = new \SplFixedArray(0);
            $allClasses = [];
            foreach ($pages as $page) {
                if (property_exists($page, 'layoutRegions') && $page->layoutRegions !== null) {
                    foreach ($page->layoutRegions as $region) {
                        if (property_exists($region, 'class')) {
                            $allClasses[$region->class] = true;
                        }
                    }
                }
            }
            foreach ($layoutClassesInclude as $expectedClass) {
                Assert::assertArrayHasKey(
                    $expectedClass,
                    $allClasses,
                    sprintf("Expected layout class '%s' not found", $expectedClass)
                );
            }
        }
    }

    public static function assertElements(
        ExtractionResult $result,
        ?int $minCount,
        ?array $typesInclude
    ): void {
        $elements = $result->elements ?? [];
        $count = count($elements);

        if ($minCount !== null) {
            Assert::assertGreaterThanOrEqual(
                $minCount,
                $count,
                sprintf("Expected at least %d elements, found %d", $minCount, $count)
            );
        }

        if ($typesInclude !== null && !empty($typesInclude)) {
            $foundTypes = [];
            foreach ($elements as $element) {
                if (isset($element->elementType)) {
                    $foundTypes[] = strtolower($element->elementType);
                }
            }

            foreach ($typesInclude as $type) {
                Assert::assertContains(
                    strtolower($type),
                    $foundTypes,
                    sprintf("Expected element type '%s' not found in %s", $type, json_encode($foundTypes))
                );
            }
        }
    }

    public static function assertMetadataExpectation(
        ExtractionResult $result,
        string $path,
        array $expectation
    ): void {
        // Convert Metadata object to array for lookup
        $metadataArray = self::metadataToArray($result->metadata);
        $value = self::lookupMetadataPath($metadataArray, $path);

        Assert::assertNotNull(
            $value,
            sprintf("Metadata path '%s' missing in %s", $path, json_encode($metadataArray))
        );

        if (isset($expectation['eq'])) {
            Assert::assertTrue(
                self::valuesEqual($value, $expectation['eq']),
                sprintf(
                    "Expected metadata '%s' == %s, got %s",
                    $path,
                    json_encode($expectation['eq']),
                    json_encode($value)
                )
            );
        }

        if (isset($expectation['gte'])) {
            Assert::assertGreaterThanOrEqual(
                (float)$expectation['gte'],
                (float)$value,
                sprintf("Expected metadata '%s' >= %s, got %s", $path, $expectation['gte'], $value)
            );
        }

        if (isset($expectation['lte'])) {
            Assert::assertLessThanOrEqual(
                (float)$expectation['lte'],
                (float)$value,
                sprintf("Expected metadata '%s' <= %s, got %s", $path, $expectation['lte'], $value)
            );
        }

        if (isset($expectation['contains'])) {
            $contains = $expectation['contains'];
            if (is_string($value) && is_string($contains)) {
                Assert::assertStringContainsString(
                    $contains,
                    $value,
                    sprintf("Expected metadata '%s' string to contain %s", $path, json_encode($contains))
                );
            } elseif (is_array($value) && is_string($contains)) {
                Assert::assertContains(
                    $contains,
                    $value,
                    sprintf("Expected metadata '%s' to contain %s", $path, json_encode($contains))
                );
            } elseif (is_array($value) && is_array($contains)) {
                $missing = array_diff($contains, $value);
                Assert::assertEmpty(
                    $missing,
                    sprintf(
                        "Expected metadata '%s' to contain %s, missing %s",
                        $path,
                        json_encode($contains),
                        json_encode($missing)
                    )
                );
            } else {
                Assert::fail(sprintf("Unsupported contains expectation for metadata '%s'", $path));
            }
        }
    }

    private static function metadataToArray($metadata): array
    {
        if (is_array($metadata)) {
            return $metadata;
        }

        // Use to_array() if available (extension Metadata object)
        if (method_exists($metadata, 'to_array')) {
            return $metadata->to_array();
        }

        // Fallback: Convert Metadata object to array using snake_case properties
        $result = [];
        $fields = [
            'language', 'subject', 'format_type', 'title', 'authors',
            'keywords', 'created_at', 'modified_at', 'created_by',
            'modified_by', 'page_count', 'sheet_count', 'format',
        ];
        foreach ($fields as $field) {
            if (isset($metadata->$field)) {
                $result[$field] = $metadata->$field;
            }
        }

        // Include custom/additional fields
        if (method_exists($metadata, 'get_additional')) {
            foreach ($metadata->get_additional() as $key => $value) {
                $result[$key] = $value;
            }
        } elseif (isset($metadata->custom) && is_array($metadata->custom)) {
            foreach ($metadata->custom as $key => $value) {
                $result[$key] = $value;
            }
        }

        return $result;
    }

    private static function lookupMetadataPath(array $metadata, string $path)
    {
        $current = $metadata;
        $segments = explode('.', $path);

        foreach ($segments as $segment) {
            if (!is_array($current) || !isset($current[$segment])) {
                // Try format metadata fallback
                if (isset($metadata['format']) && is_array($metadata['format'])) {
                    $current = $metadata['format'];
                    foreach ($segments as $seg) {
                        if (!is_array($current) || !isset($current[$seg])) {
                            return null;
                        }
                        $current = $current[$seg];
                    }
                    return $current;
                }
                return null;
            }
            $current = $current[$segment];
        }

        return $current;
    }

    private static function valuesEqual($lhs, $rhs): bool
    {
        if (is_string($lhs) && is_string($rhs)) {
            return $lhs === $rhs;
        }
        if (is_numeric($lhs) && is_numeric($rhs)) {
            return (float)$lhs === (float)$rhs;
        }
        if (is_bool($lhs) && is_bool($rhs)) {
            return $lhs === $rhs;
        }
        return $lhs == $rhs;
    }

    public static function assertDocument(
        ExtractionResult $result,
        bool $hasDocument,
        ?int $minNodeCount = null,
        ?array $nodeTypesInclude = null,
        ?bool $hasGroups = null
    ): void {
        $document = $result->document ?? null;
        if ($hasDocument) {
            Assert::assertNotNull($document, 'Expected document but got null');
            $nodes = is_array($document) ? $document : ($document->nodes ?? []);
            Assert::assertNotNull($nodes, 'Expected document.nodes but got null');
            if ($minNodeCount !== null) {
                Assert::assertGreaterThanOrEqual(
                    $minNodeCount,
                    count($nodes),
                    sprintf('Expected at least %d nodes, found %d', $minNodeCount, count($nodes))
                );
            }
            if ($nodeTypesInclude !== null && !empty($nodeTypesInclude)) {
                $foundTypes = [];
                foreach ($nodes as $node) {
                    $content = is_object($node) ? ($node->content ?? null) : ($node['content'] ?? null);
                    if ($content !== null) {
                        $nodeType = is_object($content) ? ($content->node_type ?? $content->nodeType ?? null) : ($content['node_type'] ?? null);
                        if ($nodeType !== null) {
                            $foundTypes[] = $nodeType;
                        }
                    }
                }
                foreach ($nodeTypesInclude as $type) {
                    Assert::assertContains(
                        $type,
                        $foundTypes,
                        sprintf("Expected node type '%s' not found in %s", $type, json_encode($foundTypes))
                    );
                }
            }
            if ($hasGroups !== null) {
                $hasGroupNodes = false;
                foreach ($nodes as $node) {
                    $content = is_object($node) ? ($node->content ?? null) : ($node['content'] ?? null);
                    if ($content !== null) {
                        $nodeType = is_object($content) ? ($content->node_type ?? $content->nodeType ?? null) : ($content['node_type'] ?? null);
                        if ($nodeType === 'group') {
                            $hasGroupNodes = true;
                            break;
                        }
                    }
                }
                Assert::assertEquals($hasGroups, $hasGroupNodes);
            }
        } else {
            Assert::assertNull($document, 'Expected document to be null');
        }
    }

    public static function assertOcrElements(
        ExtractionResult $result,
        ?bool $hasElements = null,
        ?bool $elementsHaveGeometry = null,
        ?bool $elementsHaveConfidence = null,
        ?int $minCount = null
    ): void {
        $ocrElements = $result->ocrElements ?? null;
        if ($hasElements) {
            Assert::assertNotNull($ocrElements, 'Expected ocr_elements but got null');
            Assert::assertIsArray($ocrElements);
            Assert::assertNotEmpty($ocrElements, 'Expected ocr_elements to be non-empty');
        }
        if (is_array($ocrElements)) {
            if ($minCount !== null) {
                Assert::assertGreaterThanOrEqual(
                    $minCount,
                    count($ocrElements),
                    sprintf('Expected at least %d ocr_elements, found %d', $minCount, count($ocrElements))
                );
            }
        }
    }

    public static function assertKeywords(
        ExtractionResult $result,
        ?bool $hasKeywords = null,
        ?int $minCount = null,
        ?int $maxCount = null
    ): void {
        if ($hasKeywords === true) {
            Assert::assertNotNull($result->extractedKeywords, 'Expected keywords but got null');
            Assert::assertNotEmpty($result->extractedKeywords ?? [], 'Expected keywords to be non-empty');
        } elseif ($hasKeywords === false) {
            $keywords = $result->extractedKeywords ?? [];
            Assert::assertTrue(
                $keywords === null || count($keywords) === 0,
                'Expected keywords to be null or empty'
            );
        }

        $keywords = $result->extractedKeywords ?? [];
        $count = count($keywords);

        if ($minCount !== null) {
            Assert::assertGreaterThanOrEqual(
                $minCount,
                $count,
                sprintf("Expected at least %d keywords, found %d", $minCount, $count)
            );
        }

        if ($maxCount !== null) {
            Assert::assertLessThanOrEqual(
                $maxCount,
                $count,
                sprintf("Expected at most %d keywords, found %d", $maxCount, $count)
            );
        }
    }

    public static function assertContentNotEmpty(ExtractionResult $result): void
    {
        Assert::assertNotEmpty(
            $result->content ?? '',
            "Expected content to be non-empty"
        );
    }

    public static function assertTableBoundingBoxes(ExtractionResult $result, bool $expected): void
    {
        if ($expected) {
            $tables = $result->tables ?? [];
            Assert::assertNotEmpty($tables, 'Expected tables with bounding boxes but no tables found');
            foreach ($tables as $table) {
                $bb = $table->boundingBox ?? $table->bounding_box ?? null;
                Assert::assertNotNull($bb, 'Expected table to have bounding_box but it was null');
            }
        }
    }

    public static function assertTableContentContainsAny(ExtractionResult $result, array $snippets): void
    {
        $tables = $result->tables ?? [];
        Assert::assertNotEmpty($tables, 'Expected tables but none found');

        $allCells = [];
        foreach ($tables as $table) {
            $cells = $table->cells ?? [];
            foreach ($cells as $row) {
                foreach ($row as $cell) {
                    $allCells[] = strtolower((string)$cell);
                }
            }
        }

        $found = false;
        foreach ($snippets as $snippet) {
            foreach ($allCells as $cell) {
                if (str_contains($cell, strtolower($snippet))) {
                    $found = true;
                    break 2;
                }
            }
        }

        Assert::assertTrue(
            $found,
            sprintf('No table cell contains any of %s', json_encode($snippets))
        );
    }

    public static function assertImageBoundingBoxes(ExtractionResult $result, bool $expected): void
    {
        if ($expected) {
            $images = $result->images ?? [];
            Assert::assertNotEmpty($images, 'Expected images with bounding boxes but no images found');
            foreach ($images as $image) {
                $bb = $image->boundingBox ?? $image->bounding_box ?? null;
                Assert::assertNotNull($bb, 'Expected image to have bounding_box but it was null');
            }
        }
    }

    public static function assertQualityScore(
        ExtractionResult $result,
        ?bool $hasScore = null,
        ?float $minScore = null,
        ?float $maxScore = null
    ): void {
        $score = $result->qualityScore ?? $result->quality_score ?? null;

        if ($hasScore === true) {
            Assert::assertNotNull($score, 'Expected quality_score to be present');
        }

        if ($hasScore === false) {
            Assert::assertNull($score, 'Expected quality_score to be absent');
        }

        if ($minScore !== null) {
            Assert::assertNotNull($score, 'quality_score required for min_score assertion');
            Assert::assertGreaterThanOrEqual(
                $minScore,
                (float)$score,
                sprintf('quality_score %f < %f', (float)$score, $minScore)
            );
        }

        if ($maxScore !== null) {
            Assert::assertNotNull($score, 'quality_score required for max_score assertion');
            Assert::assertLessThanOrEqual(
                $maxScore,
                (float)$score,
                sprintf('quality_score %f > %f', (float)$score, $maxScore)
            );
        }
    }

    public static function assertProcessingWarnings(
        ExtractionResult $result,
        ?int $maxCount = null,
        ?bool $isEmpty = null
    ): void {
        $warnings = $result->processingWarnings ?? $result->processing_warnings ?? [];

        if ($maxCount !== null) {
            Assert::assertLessThanOrEqual(
                $maxCount,
                count($warnings),
                sprintf('processing_warnings count %d > %d', count($warnings), $maxCount)
            );
        }

        if ($isEmpty === true) {
            Assert::assertCount(
                0,
                $warnings,
                sprintf('Expected empty processing_warnings, got %d', count($warnings))
            );
        }
    }

    public static function assertLlmUsage(
        ExtractionResult $result,
        ?int $maxCount = null,
        ?bool $isEmpty = null
    ): void {
        $usage = $result->llmUsage ?? $result->llm_usage ?? [];

        if ($maxCount !== null) {
            Assert::assertLessThanOrEqual(
                $maxCount,
                count($usage),
                sprintf('llm_usage count %d > %d', count($usage), $maxCount)
            );
        }

        if ($isEmpty === true) {
            Assert::assertCount(
                0,
                $usage,
                sprintf('Expected empty llm_usage, got %d', count($usage))
            );
        }
    }

    public static function assertDjotContent(
        ExtractionResult $result,
        ?bool $hasContent = null,
        ?int $minBlocks = null
    ): void {
        $djot = $result->djotContent ?? $result->djot_content ?? null;

        if ($hasContent === true) {
            Assert::assertNotNull($djot, 'Expected djot_content to be present');
        }

        if ($hasContent === false) {
            Assert::assertNull($djot, 'Expected djot_content to be absent');
        }

        if ($minBlocks !== null) {
            Assert::assertNotNull($djot, 'djot_content required for min_blocks assertion');
            $blocks = is_object($djot) ? ($djot->blocks ?? []) : ($djot['blocks'] ?? []);
            Assert::assertGreaterThanOrEqual(
                $minBlocks,
                count($blocks),
                sprintf('djot_content blocks %d < %d', count($blocks), $minBlocks)
            );
        }
    }

    public static function assertAnnotations(
        ExtractionResult $result,
        bool $hasAnnotations = false,
        ?int $minCount = null
    ): void {
        $annotations = $result->annotations ?? null;

        if ($hasAnnotations) {
            Assert::assertNotNull($annotations, 'Expected annotations to be present');
            Assert::assertIsArray($annotations);
            Assert::assertNotEmpty($annotations, 'Expected annotations to be non-empty');
        }

        if ($annotations !== null && is_array($annotations) && $minCount !== null) {
            Assert::assertGreaterThanOrEqual(
                $minCount,
                count($annotations),
                sprintf('Expected at least %d annotations, got %d', $minCount, count($annotations))
            );
        }
    }

    public static function assertStructuredOutput(
        ExtractionResult $result,
        ?bool $hasOutput = null,
        ?bool $validatesSchema = null,
        ?array $fieldExists = null
    ): void {
        $output = $result->structured_output ?? null;

        if ($hasOutput === true) {
            Assert::assertNotNull($output, 'Expected structured output to be present');
        } elseif ($hasOutput === false) {
            Assert::assertNull($output, 'Expected structured output to be absent');
        }

        if ($output !== null && $validatesSchema === true) {
            Assert::assertTrue(
                is_array($output) || is_object($output),
                'Expected structured output to validate schema'
            );
        }

        if ($output !== null && $fieldExists !== null) {
            foreach ($fieldExists as $field) {
                if (is_array($output)) {
                    Assert::assertArrayHasKey($field, $output, sprintf('Expected structured output to contain field "%s"', $field));
                } else {
                    Assert::assertTrue(
                        property_exists($output, $field),
                        sprintf('Expected structured output to contain field "%s"', $field)
                    );
                }
            }
        }
    }

    public static function skipIfFeatureUnavailable(string $feature): void
    {
        $envVar = 'KREUZBERG_' . strtoupper(str_replace('-', '_', $feature)) . '_DISABLED';
        $flag = getenv($envVar);
        if ($flag === '1' || strtolower((string) $flag) === 'true') {
            Assert::markTestSkipped(
                sprintf('Feature "%s" disabled (via %s=1)', $feature, $envVar)
            );
        }
    }

    public static function assertIsPng(string $data): void
    {
        Assert::assertGreaterThanOrEqual(4, strlen($data),
            sprintf('Data too short for PNG: %d bytes', strlen($data)));
        Assert::assertSame("\x89PNG", substr($data, 0, 4), 'Missing PNG magic bytes');
    }

    public static function assertMinByteLength(string $data, int $minLength): void
    {
        Assert::assertGreaterThanOrEqual($minLength, strlen($data),
            sprintf('Expected at least %d bytes, got %d', $minLength, strlen($data)));
    }

    public static function assertEmbedResult(array $results, int $count, int $dimensions, bool $noNan, bool $noInf, bool $nonZero): void
    {
        Assert::assertNotNull($results);
        if ($count >= 0) {
            Assert::assertCount($count, $results, sprintf("Expected %d vectors, got %d", $count, count($results)));
        }
        if (count($results) > 0) {
            foreach ($results as $i => $vector) {
                Assert::assertNotNull($vector);
                if ($dimensions > 0) {
                    Assert::assertCount($dimensions, $vector, sprintf("Vector %d expected length %d, got %d", $i, $dimensions, count($vector)));
                }

                $hasNonZero = false;
                foreach ($vector as $j => $v) {
                    if ($noNan) {
                        Assert::assertFalse(is_nan($v), sprintf("Vector %d element %d is NaN", $i, $j));
                    }
                    if ($noInf) {
                        Assert::assertFalse(is_infinite($v), sprintf("Vector %d element %d is infinite", $i, $j));
                    }
                    if ($v != 0.0) {
                        $hasNonZero = true;
                    }
                }
                if ($nonZero) {
                    Assert::assertTrue($hasNonZero, sprintf("Vector %d is all zeros", $i));
                }
            }
        }
    }
}
"#;

pub fn generate(fixtures: &[Fixture], output_dir: &Utf8Path, mode: &GenerationMode) -> Result<()> {
    let php_root = output_dir.join("php");
    let tests_dir = php_root.join("tests");
    fs::create_dir_all(&tests_dir).context("Failed to create PHP tests directory")?;
    clean_tests(&tests_dir)?;
    write_helpers(&tests_dir)?;
    write_composer_json(&php_root, mode)?;
    generate_embed_tests(fixtures, &tests_dir)?;

    let mut categories = BTreeMap::new();
    for fixture in fixtures {
        if fixture.is_document_extraction() {
            categories
                .entry(fixture.category().to_string())
                .or_insert_with(Vec::new)
                .push(fixture);
        }
    }

    for (category, fixtures) in &categories {
        let filename = format!("{}Test.php", capitalize(category));
        let content = render_category(category, fixtures)?;
        fs::write(tests_dir.join(&filename), content)
            .with_context(|| format!("Failed to write PHP test file {filename}"))?;
    }

    let api_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_plugin_api()).collect();
    if !api_fixtures.is_empty() {
        generate_plugin_api_tests(&api_fixtures, &tests_dir)?;
    }

    let render_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_render()).collect();
    if !render_fixtures.is_empty() {
        let mut sorted: Vec<_> = render_fixtures.clone();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        let content = render_render_category_php(&sorted)?;
        fs::write(tests_dir.join("RenderTest.php"), content).context("Failed to write PHP render test file")?;
    }

    write_scripts(&php_root, mode)?;

    Ok(())
}

fn write_scripts(php_root: &Utf8Path, mode: &GenerationMode) -> Result<()> {
    if !mode.is_published() {
        return Ok(());
    }
    let setup = php_root.join("setup.sh");
    fs::write(
        &setup,
        r#"#!/usr/bin/env bash
set -euo pipefail
echo "Setting up PHP test app..."
pie install kreuzberg/kreuzberg || echo "PIE install skipped (extension may already be loaded)"
composer install
echo "Setup complete."
"#,
    )
    .context("Failed to write setup.sh")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&setup, fs::Permissions::from_mode(0o755))?;
    }

    let run = php_root.join("run_tests.sh");
    fs::write(
        &run,
        r#"#!/usr/bin/env bash
set -euo pipefail
echo "Running PHP tests..."
php vendor/bin/phpunit tests/
"#,
    )
    .context("Failed to write run_tests.sh")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&run, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn write_composer_json(php_root: &Utf8Path, mode: &GenerationMode) -> Result<()> {
    let require_section = match mode {
        GenerationMode::Published { .. } => {
            r#"    "require": {
        "kreuzberg/kreuzberg": "^4.7"
    },
"#
        }
        GenerationMode::Local => "",
    };
    let content = format!(
        r#"{{
{require_section}    "autoload": {{
        "psr-4": {{
            "E2EPhp\\": "tests/"
        }}
    }},
    "autoload-dev": {{
        "psr-4": {{
            "E2EPhp\\Tests\\": "tests/"
        }}
    }},
    "require-dev": {{
        "phpunit/phpunit": "^13.1"
    }}
}}
"#
    );
    let path = php_root.join("composer.json");
    fs::write(&path, content).context("Failed to write composer.json")
}

fn clean_tests(dir: &Utf8Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir.as_std_path())? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "php") {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with("Test.php") || name == "Helpers.php" {
                fs::remove_file(entry.path())?;
            }
        }
    }

    Ok(())
}

fn write_helpers(tests_dir: &Utf8Path) -> Result<()> {
    let helpers_path = tests_dir.join("Helpers.php");
    fs::write(helpers_path.as_std_path(), PHP_HELPERS_TEMPLATE).context("Failed to write Helpers.php")?;
    Ok(())
}

fn render_category(category: &str, fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    let class_name = format!("{}Test", capitalize(category));

    writeln!(buffer, "<?php")?;
    writeln!(buffer)?;
    writeln!(buffer, "declare(strict_types=1);")?;
    writeln!(buffer)?;
    writeln!(buffer, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(
        buffer,
        "// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang php"
    )?;
    writeln!(buffer)?;
    writeln!(buffer, "// Tests for {} fixtures.", category)?;
    writeln!(buffer)?;
    writeln!(buffer, "namespace E2EPhp\\Tests;")?;
    writeln!(buffer)?;
    writeln!(buffer, "use E2EPhp\\Helpers;")?;
    writeln!(buffer, "use Kreuzberg\\Kreuzberg;")?;
    writeln!(buffer, "use PHPUnit\\Framework\\TestCase;")?;
    writeln!(buffer)?;
    writeln!(buffer, "class {} extends TestCase", class_name)?;
    writeln!(buffer, "{{")?;

    for fixture in fixtures {
        buffer.push_str(&render_test(fixture)?);
    }

    writeln!(buffer, "}}")?;

    Ok(buffer)
}

fn render_test(fixture: &Fixture) -> Result<String> {
    let mut code = String::new();
    let test_name = format!("test_{}", sanitize_identifier(&fixture.id));
    let extraction = fixture.extraction();
    let method = extraction.method;
    let input_type = extraction.input_type;

    writeln!(code, "    /**")?;
    writeln!(code, "     * {}", escape_doc_comment(&fixture.description))?;
    writeln!(code, "     */")?;
    writeln!(code, "    public function {}(): void", test_name)?;
    writeln!(code, "    {{")?;
    writeln!(
        code,
        "        $documentPath = Helpers::resolveDocument({});",
        php_string_literal(&fixture.document().path)
    )?;
    writeln!(code, "        if (!file_exists($documentPath)) {{")?;
    writeln!(
        code,
        "            $this->markTestSkipped('Skipping {}: missing document at ' . $documentPath);",
        fixture.id
    )?;
    writeln!(code, "        }}")?;
    writeln!(code)?;

    // Skip if fixture requires features that may not be available
    let doc = fixture.document();
    let skip_directive = fixture.skip();
    let all_features: Vec<&str> = skip_directive
        .requires_feature
        .iter()
        .chain(doc.requires_external_tool.iter().filter(|t| *t == "paddle-ocr"))
        .map(|s| s.as_str())
        .collect();
    for feature in &all_features {
        writeln!(
            code,
            "        Helpers::skipIfFeatureUnavailable({});",
            php_string_literal(feature)
        )?;
    }
    if !all_features.is_empty() {
        writeln!(code)?;
    }

    let config_literal = render_config_literal(&extraction.config);
    writeln!(code, "        $config = Helpers::buildConfig({});", config_literal)?;
    writeln!(code)?;

    writeln!(code, "        $kreuzberg = new Kreuzberg($config);")?;

    // Generate extraction call based on method and input_type
    match (method, input_type) {
        (ExtractionMethod::Sync, InputType::File) => {
            writeln!(code, "        $result = $kreuzberg->extractFile($documentPath);")?;
        }
        (ExtractionMethod::Async, InputType::File) => {
            writeln!(code, "        $deferred = $kreuzberg->extractFileAsync($documentPath);")?;
            writeln!(code, "        $result = $deferred->getResult();")?;
        }
        (ExtractionMethod::Sync, InputType::Bytes) => {
            writeln!(code, "        $bytes = file_get_contents($documentPath);")?;
            writeln!(code, "        $mimeType = Kreuzberg::detectMimeType($bytes);")?;
            writeln!(code, "        $result = $kreuzberg->extractBytes($bytes, $mimeType);")?;
        }
        (ExtractionMethod::Async, InputType::Bytes) => {
            writeln!(code, "        $bytes = file_get_contents($documentPath);")?;
            writeln!(code, "        $mimeType = Kreuzberg::detectMimeType($bytes);")?;
            writeln!(
                code,
                "        $deferred = $kreuzberg->extractBytesAsync($bytes, $mimeType);"
            )?;
            writeln!(code, "        $result = $deferred->getResult();")?;
        }
        (ExtractionMethod::BatchSync, InputType::File) => {
            writeln!(
                code,
                "        $results = $kreuzberg->batchExtractFiles([$documentPath]);"
            )?;
            writeln!(code, "        $result = $results[0];")?;
        }
        (ExtractionMethod::BatchAsync, InputType::File) => {
            writeln!(
                code,
                "        $deferred = $kreuzberg->batchExtractFilesAsync([$documentPath]);"
            )?;
            writeln!(code, "        $results = $deferred->getResults();")?;
            writeln!(code, "        $result = $results[0];")?;
        }
        (ExtractionMethod::BatchSync, InputType::Bytes) => {
            writeln!(code, "        $bytes = file_get_contents($documentPath);")?;
            writeln!(code, "        $mimeType = Kreuzberg::detectMimeType($bytes);")?;
            writeln!(
                code,
                "        $results = $kreuzberg->batchExtractBytes([$bytes], [$mimeType]);"
            )?;
            writeln!(code, "        $result = $results[0];")?;
        }
        (ExtractionMethod::BatchAsync, InputType::Bytes) => {
            writeln!(code, "        $bytes = file_get_contents($documentPath);")?;
            writeln!(code, "        $mimeType = Kreuzberg::detectMimeType($bytes);")?;
            writeln!(
                code,
                "        $deferred = $kreuzberg->batchExtractBytesAsync([$bytes], [$mimeType]);"
            )?;
            writeln!(code, "        $results = $deferred->getResults();")?;
            writeln!(code, "        $result = $results[0];")?;
        }
    }
    writeln!(code)?;

    code.push_str(&render_assertions(&fixture.assertions()));

    writeln!(code, "    }}")?;
    writeln!(code)?;

    Ok(code)
}

fn render_assertions(assertions: &Assertions) -> String {
    let mut buffer = String::new();

    if !assertions.expected_mime.is_empty() {
        writeln!(
            buffer,
            "        Helpers::assertExpectedMime($result, {});",
            render_string_array(&assertions.expected_mime)
        )
        .unwrap();
    }
    if let Some(min) = assertions.min_content_length {
        writeln!(buffer, "        Helpers::assertMinContentLength($result, {});", min).unwrap();
    }
    if let Some(max) = assertions.max_content_length {
        writeln!(buffer, "        Helpers::assertMaxContentLength($result, {});", max).unwrap();
    }
    if !assertions.content_contains_any.is_empty() {
        writeln!(
            buffer,
            "        Helpers::assertContentContainsAny($result, {});",
            render_string_array(&assertions.content_contains_any)
        )
        .unwrap();
    }
    if !assertions.content_contains_all.is_empty() {
        writeln!(
            buffer,
            "        Helpers::assertContentContainsAll($result, {});",
            render_string_array(&assertions.content_contains_all)
        )
        .unwrap();
    }
    if !assertions.content_contains_none.is_empty() {
        writeln!(
            buffer,
            "        Helpers::assertContentContainsNone($result, {});",
            render_string_array(&assertions.content_contains_none)
        )
        .unwrap();
    }
    if let Some(tables) = assertions.tables.as_ref() {
        let min_literal = tables.min.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        let max_literal = tables.max.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertTableCount($result, {}, {});",
            min_literal, max_literal
        )
        .unwrap();
        if let Some(has_bb) = tables.has_bounding_boxes {
            writeln!(
                buffer,
                "        Helpers::assertTableBoundingBoxes($result, {});",
                if has_bb { "true" } else { "false" }
            )
            .unwrap();
        }
        if let Some(ref contains) = tables.content_contains_any {
            writeln!(
                buffer,
                "        Helpers::assertTableContentContainsAny($result, {});",
                render_string_array(contains)
            )
            .unwrap();
        }
    }
    if let Some(languages) = assertions.detected_languages.as_ref() {
        let expected = render_string_array(&languages.expects);
        let min_conf = languages
            .min_confidence
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertDetectedLanguages($result, {}, {});",
            expected, min_conf
        )
        .unwrap();
    }
    for (path, expectation) in &assertions.metadata {
        writeln!(
            buffer,
            "        Helpers::assertMetadataExpectation($result, {}, {});",
            php_string_literal(path),
            render_php_metadata_expectation(expectation)
        )
        .unwrap();
    }
    if let Some(chunks) = assertions.chunks.as_ref() {
        let min_count = chunks
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let max_count = chunks
            .max_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let each_has_content = chunks
            .each_has_content
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        let each_has_embedding = chunks
            .each_has_embedding
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        let each_has_heading_context = chunks
            .each_has_heading_context
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        let content_starts_with_heading = chunks
            .content_starts_with_heading
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertChunks($result, {}, {}, {}, {}, {}, {});",
            min_count,
            max_count,
            each_has_content,
            each_has_embedding,
            each_has_heading_context,
            content_starts_with_heading
        )
        .unwrap();
    }
    if let Some(images) = assertions.images.as_ref() {
        let min_count = images
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let max_count = images
            .max_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let formats_include = images
            .formats_include
            .as_ref()
            .map(|v| render_string_array(v))
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertImages($result, {}, {}, {});",
            min_count, max_count, formats_include
        )
        .unwrap();
        if let Some(has_bb) = images.has_bounding_boxes {
            writeln!(
                buffer,
                "        Helpers::assertImageBoundingBoxes($result, {});",
                if has_bb { "true" } else { "false" }
            )
            .unwrap();
        }
    }
    if let Some(pages) = assertions.pages.as_ref() {
        let min_count = pages
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let exact_count = pages
            .exact_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let has_layout = pages
            .has_layout_regions
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        let layout_classes = pages
            .layout_classes_include
            .as_ref()
            .map(|c| render_string_array(c))
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertPages($result, {}, {}, {}, {});",
            min_count, exact_count, has_layout, layout_classes
        )
        .unwrap();
    }
    if let Some(elements) = assertions.elements.as_ref() {
        let min_count = elements
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let types_include = elements
            .types_include
            .as_ref()
            .map(|v| render_string_array(v))
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertElements($result, {}, {});",
            min_count, types_include
        )
        .unwrap();
    }
    if let Some(ocr) = assertions.ocr_elements.as_ref() {
        let has_elements = ocr
            .has_elements
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        let has_geometry = ocr
            .elements_have_geometry
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        let has_confidence = ocr
            .elements_have_confidence
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        let min_count = ocr
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertOcrElements($result, {}, {}, {}, {});",
            has_elements, has_geometry, has_confidence, min_count
        )
        .unwrap();
    }

    if let Some(document) = assertions.document.as_ref() {
        let has_document = if document.has_document { "true" } else { "false" };
        let min_node_count = document
            .min_node_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let node_types = if !document.node_types_include.is_empty() {
            render_string_array(&document.node_types_include)
        } else {
            "null".to_string()
        };
        let has_groups = document
            .has_groups
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertDocument($result, {}, {}, {}, {});",
            has_document, min_node_count, node_types, has_groups
        )
        .unwrap();
    }

    if let Some(keywords) = assertions.keywords.as_ref() {
        let has_keywords = keywords
            .has_keywords
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        let min_count = keywords
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let max_count = keywords
            .max_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertKeywords($result, {}, {}, {});",
            has_keywords, min_count, max_count
        )
        .unwrap();
    }

    if assertions.content_not_empty == Some(true) {
        writeln!(buffer, "        Helpers::assertContentNotEmpty($result);").unwrap();
    }

    if let Some(quality_score) = assertions.quality_score.as_ref() {
        let has_score = quality_score
            .has_score
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        let min_score = quality_score
            .min_score
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let max_score = quality_score
            .max_score
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertQualityScore($result, {}, {}, {});",
            has_score, min_score, max_score
        )
        .unwrap();
    }

    if let Some(processing_warnings) = assertions.processing_warnings.as_ref() {
        let max_count = processing_warnings
            .max_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let is_empty = processing_warnings
            .is_empty
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertProcessingWarnings($result, {}, {});",
            max_count, is_empty
        )
        .unwrap();
    }

    if let Some(llm_usage) = assertions.llm_usage.as_ref() {
        let max_count = llm_usage
            .max_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        let is_empty = llm_usage
            .is_empty
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertLlmUsage($result, {}, {});",
            max_count, is_empty
        )
        .unwrap();
    }

    if let Some(djot_content) = assertions.djot_content.as_ref() {
        let has_content = djot_content
            .has_content
            .map(|v| if v { "true" } else { "false" }.to_string())
            .unwrap_or_else(|| "null".to_string());
        let min_blocks = djot_content
            .min_blocks
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertDjotContent($result, {}, {});",
            has_content, min_blocks
        )
        .unwrap();
    }

    if let Some(annotations) = assertions.annotations.as_ref() {
        let has_annotations = if annotations.has_annotations { "true" } else { "false" };
        let min_count = annotations
            .min_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertAnnotations($result, {}, {});",
            has_annotations, min_count
        )
        .unwrap();
    }

    if let Some(structured_output) = assertions.structured_output.as_ref() {
        let has_output = structured_output
            .has_output
            .map(|v| if v { "true" } else { "false" })
            .unwrap_or("null");
        let validates_schema = structured_output
            .validates_schema
            .map(|v| if v { "true" } else { "false" })
            .unwrap_or("null");
        let field_exists = structured_output
            .field_exists
            .as_ref()
            .map(|fields| {
                let items = fields
                    .iter()
                    .map(|f| format!("'{}'", escape_php_string(f)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", items)
            })
            .unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertStructuredOutput($result, {}, {}, {});",
            has_output, validates_schema, field_exists
        )
        .unwrap();
    }

    buffer
}

fn render_config_literal(config: &Map<String, Value>) -> String {
    if config.is_empty() {
        "null".to_string()
    } else {
        let value = Value::Object(config.clone());
        render_php_value(&value)
    }
}

fn render_string_array(values: &[String]) -> String {
    if values.is_empty() {
        "[]".to_string()
    } else {
        let parts = values
            .iter()
            .map(|value| php_string_literal(value))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{parts}]")
    }
}

fn render_php_metadata_expectation(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            if map.is_empty() {
                return "[]".to_string();
            }
            let parts = map
                .iter()
                .map(|(key, value)| format!("{} => {}", php_string_literal(key), render_php_value(value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{parts}]")
        }
        _ => {
            let value_expr = render_php_value(value);
            format!("['eq' => {value_expr}]")
        }
    }
}

fn render_php_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => php_string_literal(s),
        Value::Array(items) => {
            let parts = items.iter().map(render_php_value).collect::<Vec<_>>().join(", ");
            format!("[{parts}]")
        }
        Value::Object(map) => {
            let parts = map
                .iter()
                .map(|(key, value)| format!("{} => {}", php_string_literal(key), render_php_value(value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{parts}]")
        }
    }
}

fn sanitize_identifier(input: &str) -> String {
    let mut ident = input
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' => c,
            _ => '_',
        })
        .collect::<String>();
    while ident.contains("__") {
        ident = ident.replace("__", "_");
    }
    ident.trim_matches('_').to_string()
}

fn capitalize(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

fn escape_doc_comment(value: &str) -> String {
    value.replace("*/", "* /")
}

fn escape_php_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .replace('$', "\\$")
}

fn php_string_literal(value: &str) -> String {
    format!("'{}'", escape_php_string(value))
}

/// Generate a double-quoted PHP string literal that interprets escape sequences like \n.
fn php_double_quoted_literal(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{}\"", escaped)
}

fn generate_plugin_api_tests(fixtures: &[&Fixture], output_dir: &Utf8Path) -> Result<()> {
    let test_file = output_dir.join("PluginApisTest.php");

    let mut content = String::new();

    writeln!(content, "<?php")?;
    writeln!(content)?;
    writeln!(content, "declare(strict_types=1);")?;
    writeln!(content)?;
    writeln!(content, "// Auto-generated from fixtures/plugin_api/ - DO NOT EDIT")?;
    writeln!(content)?;
    writeln!(content, "/**")?;
    writeln!(content, " * E2E tests for plugin/config/utility APIs.")?;
    writeln!(content, " *")?;
    writeln!(content, " * Generated from plugin API fixtures.")?;
    writeln!(
        content,
        " * To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang php"
    )?;
    writeln!(content, " */")?;
    writeln!(content)?;
    writeln!(content, "namespace E2EPhp\\Tests;")?;
    writeln!(content)?;
    writeln!(content, "use Kreuzberg\\Kreuzberg;")?;
    writeln!(content, "use Kreuzberg\\Config\\ExtractionConfig;")?;
    writeln!(content, "use PHPUnit\\Framework\\TestCase;")?;
    writeln!(content)?;

    let grouped = group_by_category(fixtures)?;

    writeln!(content, "class PluginApisTest extends TestCase")?;
    writeln!(content, "{{")?;

    for (_category, fixtures) in grouped {
        for fixture in fixtures {
            generate_php_test_function(fixture, &mut content)?;
        }
    }

    writeln!(content, "}}")?;

    fs::write(&test_file, content).with_context(|| format!("Failed to write {test_file}"))?;

    Ok(())
}

fn group_by_category<'a>(fixtures: &[&'a Fixture]) -> Result<BTreeMap<&'a str, Vec<&'a Fixture>>> {
    let mut grouped: BTreeMap<&str, Vec<&Fixture>> = BTreeMap::new();
    for fixture in fixtures {
        let category = fixture
            .api_category
            .as_ref()
            .with_context(|| format!("Fixture '{}' missing api_category", fixture.id))?
            .as_str();
        grouped.entry(category).or_default().push(fixture);
    }
    Ok(grouped)
}

fn generate_php_test_function(fixture: &Fixture, buf: &mut String) -> Result<()> {
    let test_spec = fixture
        .test_spec
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing test_spec", fixture.id))?;
    let test_name = format!("test_{}", fixture.id);

    writeln!(buf, "    /**")?;
    writeln!(buf, "     * {}", escape_doc_comment(&fixture.description))?;
    writeln!(buf, "     */")?;
    writeln!(buf, "    public function {}(): void", test_name)?;
    writeln!(buf, "    {{")?;

    match test_spec.pattern.as_str() {
        "simple_list" => generate_simple_list_test(fixture, test_spec, buf)?,
        "clear_registry" => generate_clear_registry_test(fixture, test_spec, buf)?,
        "graceful_unregister" => generate_graceful_unregister_test(fixture, test_spec, buf)?,
        "config_from_file" => generate_config_from_file_test(fixture, test_spec, buf)?,
        "config_discover" => generate_config_discover_test(fixture, test_spec, buf)?,
        "mime_from_bytes" => generate_mime_from_bytes_test(fixture, test_spec, buf)?,
        "mime_from_path" => generate_mime_from_path_test(fixture, test_spec, buf)?,
        "mime_extension_lookup" => generate_mime_extension_lookup_test(fixture, test_spec, buf)?,
        _ => anyhow::bail!("Unknown test pattern: {}", test_spec.pattern),
    }

    writeln!(buf, "    }}")?;
    writeln!(buf)?;
    Ok(())
}

fn generate_simple_list_test(_fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = to_camel_case(&test_spec.function_call.name);
    let assertions = &test_spec.assertions;

    writeln!(buf, "        $result = Kreuzberg::{}();", func_name)?;
    writeln!(buf, "        $this->assertIsArray($result);")?;

    if let Some(item_type) = &assertions.list_item_type
        && item_type == "string"
    {
        writeln!(buf, "        foreach ($result as $item) {{")?;
        writeln!(buf, "            $this->assertIsString($item);")?;
        writeln!(buf, "        }}")?;
    }

    if let Some(contains) = &assertions.list_contains {
        writeln!(
            buf,
            "        $this->assertContains({}, $result);",
            php_string_literal(contains)
        )?;
    }

    Ok(())
}

fn generate_clear_registry_test(_fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = to_camel_case(&test_spec.function_call.name);

    writeln!(buf, "        Kreuzberg::{}();", func_name)?;

    if test_spec.assertions.verify_cleanup {
        let list_func = func_name.replace("clear", "list");
        writeln!(buf, "        $result = Kreuzberg::{}();", list_func)?;
        writeln!(buf, "        $this->assertEmpty($result);")?;
    } else {
        writeln!(buf, "        $this->assertTrue(true); // Should not throw")?;
    }

    Ok(())
}

fn generate_graceful_unregister_test(_fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = to_camel_case(&test_spec.function_call.name);
    let arg = test_spec
        .function_call
        .args
        .first()
        .with_context(|| format!("Function '{}' missing argument", func_name))?;
    let arg_str = arg
        .as_str()
        .with_context(|| format!("Function '{}' argument is not a string", func_name))?;

    writeln!(
        buf,
        "        Kreuzberg::{}({});",
        func_name,
        php_string_literal(arg_str)
    )?;
    writeln!(buf, "        $this->assertTrue(true); // Should not throw")?;

    Ok(())
}

fn generate_config_from_file_test(fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing setup for config_from_file", fixture.id))?;
    let file_content = setup
        .temp_file_content
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_content", fixture.id))?;
    let file_name = setup
        .temp_file_name
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_name", fixture.id))?;

    writeln!(buf, "        $tmpDir = sys_get_temp_dir();")?;
    writeln!(
        buf,
        "        $configPath = $tmpDir . '/' . {};",
        php_string_literal(file_name)
    )?;
    writeln!(
        buf,
        "        file_put_contents($configPath, {});",
        php_double_quoted_literal(file_content)
    )?;
    writeln!(buf)?;

    writeln!(buf, "        $config = ExtractionConfig::fromFile($configPath);")?;
    writeln!(buf)?;

    generate_object_property_assertions(&test_spec.assertions, buf)?;

    writeln!(buf, "        unlink($configPath);")?;

    Ok(())
}

fn generate_config_discover_test(fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing setup for config_discover", fixture.id))?;
    let file_content = setup
        .temp_file_content
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_content", fixture.id))?;
    let file_name = setup
        .temp_file_name
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_name", fixture.id))?;
    let subdir = setup
        .subdirectory_name
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing subdirectory_name", fixture.id))?;

    writeln!(
        buf,
        "        $tmpDir = sys_get_temp_dir() . '/config_discover_' . uniqid();"
    )?;
    writeln!(buf, "        mkdir($tmpDir);")?;
    writeln!(
        buf,
        "        $configPath = $tmpDir . '/' . {};",
        php_string_literal(file_name)
    )?;
    writeln!(
        buf,
        "        file_put_contents($configPath, {});",
        php_double_quoted_literal(file_content)
    )?;
    writeln!(buf)?;

    writeln!(buf, "        $subdir = $tmpDir . '/' . {};", php_string_literal(subdir))?;
    writeln!(buf, "        mkdir($subdir);")?;
    writeln!(buf, "        $oldCwd = getcwd();")?;
    writeln!(buf, "        chdir($subdir);")?;
    writeln!(buf)?;

    writeln!(buf, "        $config = ExtractionConfig::discover();")?;
    writeln!(buf, "        $this->assertNotNull($config);")?;
    writeln!(buf)?;

    generate_object_property_assertions(&test_spec.assertions, buf)?;

    writeln!(buf, "        chdir($oldCwd);")?;
    writeln!(buf, "        unlink($configPath);")?;
    writeln!(buf, "        rmdir($subdir);")?;
    writeln!(buf, "        rmdir($tmpDir);")?;

    Ok(())
}

fn generate_mime_from_bytes_test(fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing setup for mime_from_bytes", fixture.id))?;
    let test_data = setup
        .test_data
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing test_data", fixture.id))?;
    let func_name = to_camel_case(&test_spec.function_call.name);

    writeln!(buf, "        $testBytes = {};", php_string_literal(test_data))?;
    writeln!(buf, "        $result = Kreuzberg::{}($testBytes);", func_name)?;
    writeln!(buf)?;

    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(
            buf,
            "        $this->assertStringContainsStringIgnoringCase({}, $result);",
            php_string_literal(contains)
        )?;
    }

    Ok(())
}

fn generate_mime_from_path_test(fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let setup = test_spec
        .setup
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing setup for mime_from_path", fixture.id))?;
    let file_name = setup
        .temp_file_name
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_name", fixture.id))?;
    let file_content = setup
        .temp_file_content
        .as_ref()
        .with_context(|| format!("Fixture '{}' missing temp_file_content", fixture.id))?;
    let func_name = to_camel_case(&test_spec.function_call.name);

    writeln!(buf, "        $tmpDir = sys_get_temp_dir();")?;
    writeln!(
        buf,
        "        $testFile = $tmpDir . '/' . {};",
        php_string_literal(file_name)
    )?;
    writeln!(
        buf,
        "        file_put_contents($testFile, {});",
        php_string_literal(file_content)
    )?;
    writeln!(buf)?;

    writeln!(buf, "        $result = Kreuzberg::{}($testFile);", func_name)?;
    writeln!(buf)?;

    if let Some(contains) = &test_spec.assertions.string_contains {
        writeln!(
            buf,
            "        $this->assertStringContainsStringIgnoringCase({}, $result);",
            php_string_literal(contains)
        )?;
    }

    writeln!(buf, "        unlink($testFile);")?;

    Ok(())
}

fn generate_mime_extension_lookup_test(_fixture: &Fixture, test_spec: &PluginTestSpec, buf: &mut String) -> Result<()> {
    let func_name = to_camel_case(&test_spec.function_call.name);
    let arg = test_spec
        .function_call
        .args
        .first()
        .with_context(|| format!("Function '{}' missing argument", func_name))?;
    let mime_type = arg
        .as_str()
        .with_context(|| format!("Function '{}' argument is not a string", func_name))?;

    writeln!(
        buf,
        "        $result = Kreuzberg::{}({});",
        func_name,
        php_string_literal(mime_type)
    )?;
    writeln!(buf, "        $this->assertIsArray($result);")?;

    if let Some(contains) = &test_spec.assertions.list_contains {
        writeln!(
            buf,
            "        $this->assertContains({}, $result);",
            php_string_literal(contains)
        )?;
    }

    Ok(())
}

fn generate_object_property_assertions(assertions: &PluginAssertions, buf: &mut String) -> Result<()> {
    for prop in &assertions.object_properties {
        let parts: Vec<&str> = prop.path.split('.').collect();
        // Convert each path segment from snake_case to camelCase for PHP property access
        let camel_parts: Vec<String> = parts.iter().map(|p| to_camel_case(p)).collect();
        let php_path = format!("$config->{}", camel_parts.join("->"));

        if let Some(exists) = prop.exists
            && exists
        {
            writeln!(buf, "        $this->assertNotNull({});", php_path)?;
        }

        if let Some(value) = &prop.value {
            match value {
                Value::Number(n) => writeln!(buf, "        $this->assertEquals({}, {});", n, php_path)?,
                Value::Bool(b) => {
                    let bool_str = if *b { "true" } else { "false" };
                    writeln!(buf, "        $this->assertEquals({}, {});", bool_str, php_path)?
                }
                Value::String(s) => writeln!(
                    buf,
                    "        $this->assertEquals({}, {});",
                    php_string_literal(s),
                    php_path
                )?,
                _ => {}
            }
        }
    }

    Ok(())
}

fn to_camel_case(snake_case: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for (i, ch) in snake_case.chars().enumerate() {
        if ch == '_' {
            capitalize_next = true;
        } else if i == 0 {
            // First character is always lowercase for camelCase
            result.push(ch.to_ascii_lowercase());
        } else if capitalize_next {
            // Capitalize after underscore
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

fn render_render_category_php(fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "<?php")?;
    writeln!(buffer)?;
    writeln!(buffer, "declare(strict_types=1);")?;
    writeln!(buffer)?;
    writeln!(buffer, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(
        buffer,
        "// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang php"
    )?;
    writeln!(buffer)?;
    writeln!(buffer, "// Tests for render fixtures.")?;
    writeln!(buffer)?;
    writeln!(buffer, "namespace E2EPhp\\Tests;")?;
    writeln!(buffer)?;
    writeln!(buffer, "use E2EPhp\\Helpers;")?;
    writeln!(buffer, "use PHPUnit\\Framework\\TestCase;")?;
    writeln!(buffer)?;
    writeln!(buffer, "use function Kreuzberg\\render_pdf_page;")?;
    writeln!(buffer, "use function Kreuzberg\\render_pdf_pages_iter;")?;
    writeln!(buffer)?;
    writeln!(buffer, "class RenderTest extends TestCase")?;
    writeln!(buffer, "{{")?;

    for fixture in fixtures {
        buffer.push_str(&render_render_test_php(fixture)?);
        writeln!(buffer)?;
    }

    writeln!(buffer, "}}")?;
    Ok(buffer)
}

fn render_render_test_php(fixture: &Fixture) -> Result<String> {
    let mut code = String::new();
    let render = fixture.render.as_ref().expect("render spec required");
    let assertions = fixture.assertions().render.unwrap_or_default();
    let method_name = format!("test_{}", sanitize_identifier(&fixture.id));
    let doc_path = escape_php_string(&fixture.document().path);

    writeln!(code, "    public function {method_name}(): void")?;
    writeln!(code, "    {{")?;
    writeln!(code, "        $documentPath = Helpers::resolveDocument('{doc_path}');")?;
    writeln!(code, "        if (!file_exists($documentPath)) {{")?;
    writeln!(
        code,
        "            $this->markTestSkipped('Missing document: ' . $documentPath);"
    )?;
    writeln!(code, "        }}")?;

    let dpi_arg = render.dpi.map(|d| format!(", {d}")).unwrap_or_default();

    match render.mode.as_str() {
        "single_page" => {
            let page_index = render.page_index.unwrap_or(0);
            writeln!(
                code,
                "        $pngData = render_pdf_page($documentPath, {page_index}{dpi_arg});"
            )?;
            writeln!(
                code,
                "        if (is_array($pngData)) {{ $pngData = pack('C*', ...$pngData); }}"
            )?;
            render_render_assertions_php(&assertions, "$pngData", &mut code, "        ")?;
        }
        "iterator" => {
            writeln!(code, "        $pages = [];")?;
            writeln!(
                code,
                "        foreach (render_pdf_pages_iter($documentPath{dpi_arg}) as $pngData) {{"
            )?;
            writeln!(
                code,
                "            if (is_array($pngData)) {{ $pngData = pack('C*', ...$pngData); }}"
            )?;
            writeln!(code, "            Helpers::assertIsPng($pngData);")?;
            writeln!(code, "            $pages[] = $pngData;")?;
            writeln!(code, "        }}")?;
            if let Some(page_count_gte) = assertions.page_count_gte {
                writeln!(
                    code,
                    "        $this->assertGreaterThanOrEqual({page_count_gte}, count($pages),"
                )?;
                writeln!(
                    code,
                    "            sprintf('Expected at least {page_count_gte} pages, got %d', count($pages)));"
                )?;
            }
        }
        _ => anyhow::bail!("Unknown render mode: {}", render.mode),
    }

    writeln!(code, "    }}")?;
    Ok(code)
}

fn render_render_assertions_php(
    assertions: &RenderAssertions,
    var: &str,
    code: &mut String,
    indent: &str,
) -> Result<()> {
    if assertions.is_png == Some(true) {
        writeln!(code, "{indent}Helpers::assertIsPng({var});")?;
    }
    if let Some(min_len) = assertions.min_byte_length {
        writeln!(code, "{indent}Helpers::assertMinByteLength({var}, {min_len});")?;
    }
    Ok(())
}

pub fn generate_parity(manifest: &ParityManifest, output_root: &Utf8Path, _mode: &GenerationMode) -> Result<()> {
    let php_root = output_root.join("php");
    let tests_dir = php_root.join("tests");
    fs::create_dir_all(&tests_dir).context("Failed to create PHP tests directory")?;

    let lang = "php";
    let mut buf = String::new();

    writeln!(buf, "<?php")?;
    writeln!(buf)?;
    writeln!(buf, "declare(strict_types=1);")?;
    writeln!(buf)?;
    writeln!(buf, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(
        buf,
        "// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang php"
    )?;
    writeln!(buf)?;
    writeln!(buf, "use PHPUnit\\Framework\\TestCase;")?;
    writeln!(buf, "use PHPUnit\\Framework\\Attributes\\Test;")?;
    writeln!(buf)?;
    writeln!(buf, "class ParityTest extends TestCase")?;
    writeln!(buf, "{{")?;

    // Collect all struct types that have fields for this language
    let mut struct_types: Vec<(&String, std::collections::BTreeMap<String, parity::FieldDef>)> = Vec::new();
    for (type_name, type_def) in &manifest.types {
        if let TypeDef::Struct { .. } = type_def
            && let Some(fields) = parity::fields_for_type_and_lang(manifest, type_name, lang)
        {
            struct_types.push((type_name, fields));
        }
    }

    // Emit a test for each struct type
    for (type_name, fields) in &struct_types {
        let fn_name = format!("{}_has_all_expected_properties", parity::to_snake_case(type_name));
        let all_fields: Vec<String> = fields.keys().map(|name| parity::to_camel_case(name)).collect();

        writeln!(buf, "    #[Test]")?;
        writeln!(buf, "    public function {fn_name}(): void")?;
        writeln!(buf, "    {{")?;
        writeln!(
            buf,
            "        $ref = new \\ReflectionClass(\\Kreuzberg\\Types\\{type_name}::class);"
        )?;
        writeln!(buf, "        $expected = [")?;
        for field in &all_fields {
            writeln!(buf, "            '{field}',")?;
        }
        writeln!(buf, "        ];")?;
        writeln!(buf, "        foreach ($expected as $prop) {{")?;
        writeln!(
            buf,
            "            // hasProperty() only covers #[php_prop] declared fields."
        )?;
        writeln!(
            buf,
            "            // Virtual properties backed by #[php(getter)] methods are"
        )?;
        writeln!(
            buf,
            "            // accessible via $obj->prop but only show up as a getter method"
        )?;
        writeln!(
            buf,
            "            // (getXxx) in ReflectionClass, not as a declared property."
        )?;
        writeln!(buf, "            $getter = 'get' . ucfirst($prop);")?;
        writeln!(buf, "            $this->assertTrue(")?;
        writeln!(
            buf,
            "                $ref->hasProperty($prop) || $ref->hasMethod($getter),"
        )?;
        writeln!(
            buf,
            "                \"{type_name} missing property or getter: {{$prop}}\""
        )?;
        writeln!(buf, "            );")?;
        writeln!(buf, "        }}")?;
        writeln!(buf, "    }}")?;
        writeln!(buf)?;
    }

    // Emit tests for simple enums
    for (type_name, type_def) in &manifest.types {
        if let TypeDef::SimpleEnum { values } = type_def {
            let fn_name = format!("{}_has_all_expected_values", parity::to_snake_case(type_name));
            writeln!(buf, "    #[Test]")?;
            writeln!(buf, "    public function {fn_name}(): void")?;
            writeln!(buf, "    {{")?;
            writeln!(
                buf,
                "        $ref = new \\ReflectionEnum(\\Kreuzberg\\Types\\{type_name}::class);"
            )?;
            writeln!(buf, "        $expected = [")?;
            for value in values {
                let camel = parity::to_camel_case(value);
                writeln!(buf, "            '{camel}',")?;
            }
            writeln!(buf, "        ];")?;
            writeln!(buf, "        $cases = array_map(")?;
            writeln!(buf, "            fn($case) => $case->name,")?;
            writeln!(buf, "            $ref->getCases()")?;
            writeln!(buf, "        );")?;
            writeln!(buf, "        foreach ($expected as $val) {{")?;
            writeln!(buf, "            $this->assertContains(")?;
            writeln!(buf, "                $val,")?;
            writeln!(buf, "                $cases,")?;
            writeln!(buf, "                \"{type_name} missing enum value: {{$val}}\"")?;
            writeln!(buf, "            );")?;
            writeln!(buf, "        }}")?;
            writeln!(buf, "    }}")?;
            writeln!(buf)?;
        }
    }

    writeln!(buf, "}}")?;

    let path = tests_dir.join("ParityTest.php");
    fs::write(&path, &buf).with_context(|| format!("Writing {path}"))?;

    Ok(())
}

fn generate_embed_tests(fixtures: &[Fixture], output_dir: &Utf8Path) -> Result<()> {
    let embed_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_embed()).collect();
    if embed_fixtures.is_empty() {
        return Ok(());
    }

    let content = render_embed_category_php(&embed_fixtures)?;
    let test_file = output_dir.join("EmbedStandaloneTest.php");
    fs::write(&test_file, content).with_context(|| format!("Failed to write {test_file}"))?;
    Ok(())
}

fn render_embed_category_php(fixtures: &[&Fixture]) -> Result<String> {
    let mut buffer = String::new();
    writeln!(buffer, "<?php")?;
    writeln!(buffer)?;
    writeln!(buffer, "declare(strict_types=1);")?;
    writeln!(buffer)?;
    writeln!(buffer, "// Code generated by kreuzberg-e2e-generator. DO NOT EDIT.")?;
    writeln!(
        buffer,
        "// To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang php"
    )?;
    writeln!(buffer)?;
    writeln!(buffer, "// Tests for embedding fixtures.")?;
    writeln!(buffer)?;
    writeln!(buffer, "namespace E2EPhp\\Tests;")?;
    writeln!(buffer)?;
    writeln!(buffer, "use E2EPhp\\Helpers;")?;
    writeln!(buffer, "use Kreuzberg\\Kreuzberg;")?;
    writeln!(buffer, "use PHPUnit\\Framework\\TestCase;")?;
    writeln!(buffer)?;
    writeln!(buffer, "class EmbedStandaloneTest extends TestCase")?;
    writeln!(buffer, "{{")?;

    for fixture in fixtures {
        buffer.push_str(&render_embed_test_php(fixture)?);
        writeln!(buffer)?;
    }

    writeln!(buffer, "}}")?;
    Ok(buffer)
}

fn render_embed_test_php(fixture: &Fixture) -> Result<String> {
    let mut code = String::new();
    let embed = fixture.embed.as_ref().expect("embed spec required");
    let fixture_assertions = fixture.assertions();
    let assertions = fixture_assertions.embed.as_ref().expect("embed assertions required");
    let method_name = format!("test_{}", sanitize_identifier(&fixture.id));

    writeln!(code, "    public function {method_name}(): void")?;
    writeln!(code, "    {{")?;

    if !fixture.id.contains("disabled") {
        writeln!(
            code,
            "        if (PHP_OS_FAMILY === 'Windows' && php_uname('m') === 'x86_64') {{"
        )?;
        writeln!(
            code,
            "            $this->markTestSkipped('Skip embeddings on Windows X64 until ONNX is implemented');"
        )?;
        writeln!(code, "        }}")?;
    }

    let texts_arr = render_php_string_list(&embed.texts);
    let config_expr = if !embed.config.is_empty() {
        render_embed_config_php(&embed.config)?
    } else {
        "null".to_string()
    };

    let func = if embed.async_variant { "embedAsync" } else { "embed" };

    writeln!(code, "        try {{")?;
    writeln!(
        code,
        "            $results = Kreuzberg::{func}({texts_arr}, {config_expr});"
    )?;
    writeln!(code, "        }} catch (\\Exception $e) {{")?;
    writeln!(
        code,
        "            $this->markTestSkipped('Embeddings not supported or requirements missing: ' . $e->getMessage());"
    )?;
    writeln!(code, "            return;")?;
    writeln!(code, "        }}")?;

    let count = assertions.count.map(|c| c as i32).unwrap_or(-1);
    let dimensions = assertions.dimensions.map(|d| d as i32).unwrap_or(-1);

    writeln!(
        code,
        "        Helpers::assertEmbedResult($results, {}, {}, {}, {}, {});",
        count,
        dimensions,
        if assertions.no_nan { "true" } else { "false" },
        if assertions.no_inf { "true" } else { "false" },
        if assertions.non_zero { "true" } else { "false" }
    )?;

    writeln!(code, "    }}")?;
    Ok(code)
}

fn render_embed_config_php(config: &Map<String, Value>) -> Result<String> {
    let mut parts = Vec::new();

    if let Some(model) = config.get("model")
        && let Some(name) = model.get("name").and_then(|v| v.as_str())
    {
        parts.push(format!("'model' => '{}'", name));
    }

    if let Some(v) = config.get("normalize").and_then(|v| v.as_bool()) {
        parts.push(format!("'normalize' => {}", if v { "true" } else { "false" }));
    }

    if let Some(v) = config.get("batch_size").and_then(|v| v.as_i64()) {
        parts.push(format!("'batch_size' => {v}"));
    }

    Ok(format!("[{}]", parts.join(", ")))
}

fn render_php_string_list(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|s| php_string_literal(s)).collect();
    format!("[{}]", parts.join(", "))
}
