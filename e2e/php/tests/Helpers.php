<?php

declare(strict_types=1);

namespace E2EPhp;

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Config\KeywordConfig;
use Kreuzberg\Config\LanguageDetectionConfig;
use Kreuzberg\Config\PdfConfig;
use Kreuzberg\Config\PostProcessorConfig;
use Kreuzberg\Config\TokenReductionConfig;
use Kreuzberg\ExtractionResult;
use PHPUnit\Framework\Assert;

class Helpers
{
    private static ?string $workspaceRoot = null;
    private static ?string $testDocuments = null;

    public static function getWorkspaceRoot(): string
    {
        if (self::$workspaceRoot === null) {
            self::$workspaceRoot = realpath(__DIR__ . '/../../..');
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

        $params = [];

        // Handle nested config objects
        if (isset($config['ocr']) && is_array($config['ocr'])) {
            $ocrParams = [];
            if (isset($config['ocr']['backend'])) {
                $ocrParams['backend'] = $config['ocr']['backend'];
            }
            if (isset($config['ocr']['language'])) {
                $ocrParams['language'] = $config['ocr']['language'];
            }
            if (!empty($ocrParams)) {
                $params['ocr'] = new OcrConfig(...$ocrParams);
            }
        }
        if (isset($config['chunking']) && is_array($config['chunking'])) {
            $params['chunking'] = new ChunkingConfig(...$config['chunking']);
        }
        if (isset($config['images']) && is_array($config['images'])) {
            $params['imageExtraction'] = new ImageExtractionConfig(...$config['images']);
        }
        if (isset($config['pdf_options']) && is_array($config['pdf_options'])) {
            $params['pdf'] = new PdfConfig(...$config['pdf_options']);
        }
        if (isset($config['language_detection']) && is_array($config['language_detection'])) {
            $params['languageDetection'] = new LanguageDetectionConfig(...$config['language_detection']);
        }
        if (isset($config['keywords']) && is_array($config['keywords'])) {
            $params['keywords'] = new KeywordConfig(...$config['keywords']);
        }
        if (isset($config['postprocessor']) && is_array($config['postprocessor'])) {
            $params['postprocessor'] = new PostProcessorConfig(...$config['postprocessor']);
        }
        if (isset($config['token_reduction']) && is_array($config['token_reduction'])) {
            $params['tokenReduction'] = new TokenReductionConfig(...$config['token_reduction']);
        }

        // Handle scalar config options
        if (isset($config['use_cache'])) {
            $params['useCache'] = (bool)$config['use_cache'];
        }
        if (isset($config['force_ocr'])) {
            $params['forceOcr'] = (bool)$config['force_ocr'];
        }
        if (isset($config['enable_quality_processing'])) {
            $params['enableQualityProcessing'] = (bool)$config['enable_quality_processing'];
        }
        if (isset($config['include_document_structure'])) {
            $params['includeDocumentStructure'] = (bool)$config['include_document_structure'];
        }
        if (isset($config['output_format'])) {
            $params['outputFormat'] = $config['output_format'];
        }
        if (isset($config['result_format'])) {
            $params['resultFormat'] = $config['result_format'];
        }

        return new ExtractionConfig(...$params);
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

        if ($minConfidence !== null && isset($result->metadata['confidence'])) {
            $confidence = $result->metadata['confidence'];
            Assert::assertGreaterThanOrEqual(
                $minConfidence,
                $confidence,
                sprintf("Expected confidence >= %f, got %f", $minConfidence, $confidence)
            );
        }
    }

    public static function assertChunks(
        ExtractionResult $result,
        ?int $minCount,
        ?int $maxCount,
        ?bool $eachHasContent,
        ?bool $eachHasEmbedding
    ): void {
        $chunks = $result->chunks ?? [];
        $count = count($chunks);

        if ($minCount !== null) {
            Assert::assertGreaterThanOrEqual(
                $minCount,
                $count,
                sprintf("Expected at least %d chunks, found %d", $minCount, $count)
            );
        }

        if ($maxCount !== null) {
            Assert::assertLessThanOrEqual(
                $maxCount,
                $count,
                sprintf("Expected at most %d chunks, found %d", $maxCount, $count)
            );
        }

        if ($eachHasContent === true) {
            foreach ($chunks as $i => $chunk) {
                Assert::assertNotEmpty(
                    $chunk->content ?? '',
                    sprintf("Chunk %d should have content", $i)
                );
            }
        }

        if ($eachHasEmbedding === true) {
            foreach ($chunks as $i => $chunk) {
                Assert::assertNotNull(
                    $chunk->embedding ?? null,
                    sprintf("Chunk %d should have embedding", $i)
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
                if (isset($element->type)) {
                    $foundTypes[] = strtolower($element->type);
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

        // Convert Metadata object to array
        $result = [];
        if (isset($metadata->language)) {
            $result['language'] = $metadata->language;
        }
        if (isset($metadata->date)) {
            $result['date'] = $metadata->date;
        }
        if (isset($metadata->subject)) {
            $result['subject'] = $metadata->subject;
        }
        if (isset($metadata->formatType)) {
            $result['format_type'] = $metadata->formatType;
        }
        if (isset($metadata->title)) {
            $result['title'] = $metadata->title;
        }
        if (isset($metadata->authors)) {
            $result['authors'] = $metadata->authors;
        }
        if (isset($metadata->keywords)) {
            $result['keywords'] = $metadata->keywords;
        }
        if (isset($metadata->createdAt)) {
            $result['created_at'] = $metadata->createdAt;
        }
        if (isset($metadata->modifiedAt)) {
            $result['modified_at'] = $metadata->modifiedAt;
        }
        if (isset($metadata->createdBy)) {
            $result['created_by'] = $metadata->createdBy;
        }
        if (isset($metadata->producer)) {
            $result['producer'] = $metadata->producer;
        }
        if (isset($metadata->pageCount)) {
            $result['page_count'] = $metadata->pageCount;
        }
        if (isset($metadata->custom) && is_array($metadata->custom)) {
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
}
