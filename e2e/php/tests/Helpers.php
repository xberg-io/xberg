<?php

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
                if (empty($chunk->metadata->heading_context ?? null)) {
                    continue;
                }
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
        ?int $exactCount
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

    public static function skipIfFeatureUnavailable(string $feature): void
    {
        $envVar = 'KREUZBERG_' . strtoupper(str_replace('-', '_', $feature)) . '_AVAILABLE';
        $flag = getenv($envVar);
        if ($flag === false || $flag === '' || $flag === '0' || strtolower($flag) === 'false') {
            Assert::markTestSkipped(
                sprintf('Feature "%s" not available (set %s=1)', $feature, $envVar)
            );
        }
    }

    public static function assertIsPng(string $data): void
    {
        Assert::assertGreaterThanOrEqual(
            4,
            strlen($data),
            sprintf('Data too short for PNG: %d bytes', strlen($data))
        );
        Assert::assertSame("\x89PNG", substr($data, 0, 4), 'Missing PNG magic bytes');
    }

    public static function assertMinByteLength(string $data, int $minLength): void
    {
        Assert::assertGreaterThanOrEqual(
            $minLength,
            strlen($data),
            sprintf('Expected at least %d bytes, got %d', $minLength, strlen($data))
        );
    }
}
