use crate::fixtures::{Assertions, ExtractionMethod, Fixture, InputType, PluginAssertions, PluginTestSpec};
use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;

const PHP_HELPERS_TEMPLATE: &str = r#"<?php

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
"#;

pub fn generate(fixtures: &[Fixture], output_root: &Utf8Path) -> Result<()> {
    let php_root = output_root.join("php");
    let tests_dir = php_root.join("tests");

    fs::create_dir_all(&tests_dir).context("Failed to create PHP tests directory")?;

    clean_tests(&tests_dir)?;
    write_helpers(&tests_dir)?;

    let doc_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_document_extraction()).collect();
    let api_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_plugin_api()).collect();

    let mut grouped = doc_fixtures
        .into_iter()
        .into_group_map_by(|fixture| fixture.category().to_string())
        .into_iter()
        .collect::<Vec<_>>();
    grouped.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, mut fixtures) in grouped {
        fixtures.sort_by(|a, b| a.id.cmp(&b.id));
        let filename = format!("{}Test.php", capitalize(&category));
        let content = render_category(&category, &fixtures)?;
        fs::write(tests_dir.join(&filename), content)
            .with_context(|| format!("Failed to write PHP test file {filename}"))?;
    }

    if !api_fixtures.is_empty() {
        generate_plugin_api_tests(&api_fixtures, &tests_dir)?;
    }

    Ok(())
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

    let config_literal = render_config_literal(&extraction.config);
    writeln!(code, "        $config = Helpers::buildConfig({});", config_literal)?;
    writeln!(code)?;

    writeln!(code, "        $kreuzberg = new Kreuzberg($config);")?;

    // Generate extraction call based on method and input_type
    // Note: PHP SDK does not have async methods - all operations are synchronous.
    // Async/BatchAsync methods map to their sync equivalents.
    match (method, input_type) {
        (ExtractionMethod::Sync, InputType::File) | (ExtractionMethod::Async, InputType::File) => {
            writeln!(code, "        $result = $kreuzberg->extractFile($documentPath);")?;
        }
        (ExtractionMethod::Sync, InputType::Bytes) | (ExtractionMethod::Async, InputType::Bytes) => {
            writeln!(code, "        $bytes = file_get_contents($documentPath);")?;
            writeln!(code, "        $mimeType = Kreuzberg::detectMimeType($bytes);")?;
            writeln!(code, "        $result = $kreuzberg->extractBytes($bytes, $mimeType);")?;
        }
        (ExtractionMethod::BatchSync, InputType::File) | (ExtractionMethod::BatchAsync, InputType::File) => {
            writeln!(
                code,
                "        $results = $kreuzberg->batchExtractFiles([$documentPath]);"
            )?;
            writeln!(code, "        $result = $results[0];")?;
        }
        (ExtractionMethod::BatchSync, InputType::Bytes) | (ExtractionMethod::BatchAsync, InputType::Bytes) => {
            writeln!(code, "        $bytes = file_get_contents($documentPath);")?;
            writeln!(code, "        $mimeType = Kreuzberg::detectMimeType($bytes);")?;
            writeln!(
                code,
                "        $results = $kreuzberg->batchExtractBytes([$bytes], [$mimeType]);"
            )?;
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
    if let Some(tables) = assertions.tables.as_ref() {
        let min_literal = tables.min.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        let max_literal = tables.max.map(|v| v.to_string()).unwrap_or_else(|| "null".to_string());
        writeln!(
            buffer,
            "        Helpers::assertTableCount($result, {}, {});",
            min_literal, max_literal
        )
        .unwrap();
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
        writeln!(
            buffer,
            "        Helpers::assertChunks($result, {}, {}, {}, {});",
            min_count, max_count, each_has_content, each_has_embedding
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
        writeln!(
            buffer,
            "        Helpers::assertPages($result, {}, {});",
            min_count, exact_count
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

    let list_func = func_name.replace("clear", "list");
    writeln!(buf, "        $result = Kreuzberg::{}();", list_func)?;
    writeln!(buf, "        $this->assertEmpty($result);")?;

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
        php_string_literal(file_content)
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
        php_string_literal(file_content)
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
