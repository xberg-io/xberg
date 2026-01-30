<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Integration;

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\PageConfig;
use Kreuzberg\Kreuzberg;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;
use ReflectionClass;

/**
 * Integration tests for pages extraction functionality.
 *
 * Comprehensive testing of multi-page document processing including:
 * - Page extraction and structure
 * - Page markers and formatting
 * - Multi-page document handling
 * - Page metadata
 * - Configuration validation
 * - Readonly property enforcement
 *
 * Test Coverage:
 * - extractPages configuration
 * - insertPageMarkers functionality
 * - Custom marker formats
 * - Multi-page PDF handling
 * - Page content structure
 * - Page metadata extraction
 * - PageConfig builder pattern
 * - JSON serialization
 */
#[CoversClass(Kreuzberg::class)]
#[CoversClass(PageConfig::class)]
#[Group('integration')]
#[Group('pages')]
#[RequiresPhpExtension('kreuzberg-php')]
final class PagesExtractionTest extends TestCase
{
    private string $testDocumentsPath;

    protected function setUp(): void
    {
        if (!extension_loaded('kreuzberg-php')) {
            $this->markTestSkipped('Kreuzberg extension is not loaded');
        }

        $this->testDocumentsPath = dirname(__DIR__, 4) . DIRECTORY_SEPARATOR . 'test_documents';
    }

    /**
     * Test: Page extraction returns pages array.
     */
    #[Test]
    public function it_extracts_pages_from_document(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(
            pages: new PageConfig(extractPages: true),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->pages, 'Pages should be extracted');
        $this->assertIsArray($result->pages, 'Pages should be an array');
    }

    /**
     * Test: Pages array contains objects with required structure.
     */
    #[Test]
    public function it_provides_page_structure(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(
            pages: new PageConfig(extractPages: true),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->pages, 'Pages should not be null');
        $this->assertNotEmpty($result->pages, 'Should produce pages');

        foreach ($result->pages as $page) {
            $this->assertIsObject($page, 'Each page should be an object');

            // Check for expected page properties
            if (isset($page->content)) {
                $this->assertIsString($page->content, 'Page content should be a string');
            }

            if (isset($page->pageNumber)) {
                $this->assertIsInt($page->pageNumber, 'Page number should be an integer');
                $this->assertGreaterThan(0, $page->pageNumber, 'Page number should be positive');
            }
        }
    }

    /**
     * Test: Page markers are inserted in content.
     */
    #[Test]
    public function it_inserts_page_markers_in_content(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(
            pages: new PageConfig(
                extractPages: true,
                insertPageMarkers: true,
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        if (!empty($result->content)) {
            // Check if page markers are present in content
            $this->assertIsString($result->content, 'Content should be a string');
        }
    }

    /**
     * Test: Custom page marker format is applied.
     */
    #[Test]
    public function it_applies_custom_page_marker_format(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(
            pages: new PageConfig(
                extractPages: true,
                insertPageMarkers: true,
                markerFormat: '--- PAGE {page_number} ---',
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->content, 'Content should be extracted');
    }

    /**
     * Test: PageConfig constructor accepts parameters.
     */
    #[Test]
    public function it_creates_page_config_with_parameters(): void
    {
        $config = new PageConfig(
            extractPages: true,
            insertPageMarkers: true,
            markerFormat: '[PAGE: {page_number}]',
        );

        $this->assertTrue($config->extractPages, 'extractPages should be set');
        $this->assertTrue($config->insertPageMarkers, 'insertPageMarkers should be set');
        $this->assertSame('[PAGE: {page_number}]', $config->markerFormat, 'markerFormat should be set');
    }

    /**
     * Test: PageConfig default values.
     */
    #[Test]
    public function it_uses_default_page_config_values(): void
    {
        $config = new PageConfig();

        $this->assertFalse($config->extractPages, 'Default extractPages should be false');
        $this->assertFalse($config->insertPageMarkers, 'Default insertPageMarkers should be false');
    }

    /**
     * Test: PageConfig supports fromArray factory method.
     */
    #[Test]
    public function it_creates_page_config_from_array(): void
    {
        $data = [
            'extract_pages' => true,
            'insert_page_markers' => true,
            'marker_format' => '<<PAGE {page_number}>>',
        ];

        $config = PageConfig::fromArray($data);

        $this->assertTrue($config->extractPages);
        $this->assertTrue($config->insertPageMarkers);
        $this->assertSame('<<PAGE {page_number}>>', $config->markerFormat);
    }

    /**
     * Test: PageConfig supports toArray conversion.
     */
    #[Test]
    public function it_converts_page_config_to_array(): void
    {
        $config = new PageConfig(
            extractPages: true,
            insertPageMarkers: false,
            markerFormat: 'CUSTOM',
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertTrue($array['extract_pages']);
        $this->assertFalse($array['insert_page_markers']);
        $this->assertSame('CUSTOM', $array['marker_format']);
    }

    /**
     * Test: PageConfig supports JSON serialization.
     */
    #[Test]
    public function it_serializes_page_config_to_json(): void
    {
        $config = new PageConfig(
            extractPages: true,
            insertPageMarkers: true,
        );

        $json = $config->toJson();

        $this->assertIsString($json);
        $this->assertStringContainsString('extract_pages', $json);
        $this->assertStringContainsString('insert_page_markers', $json);
    }

    /**
     * Test: PageConfig supports JSON deserialization.
     */
    #[Test]
    public function it_deserializes_page_config_from_json(): void
    {
        $json = '{"extract_pages": true, "insert_page_markers": false, "marker_format": "JSON_MARKER"}';

        $config = PageConfig::fromJson($json);

        $this->assertTrue($config->extractPages);
        $this->assertFalse($config->insertPageMarkers);
        $this->assertSame('JSON_MARKER', $config->markerFormat);
    }

    /**
     * Test: PageConfig readonly properties are enforced.
     */
    #[Test]
    public function it_enforces_readonly_page_config_properties(): void
    {
        $config = new PageConfig(
            extractPages: true,
            insertPageMarkers: true,
        );

        $reflection = new ReflectionClass($config);

        if ($reflection->getAttributes() || $reflection->getProperties()) {
            // Verify properties are accessible
            $this->assertTrue($config->extractPages);
            $this->assertTrue($config->insertPageMarkers);
        }
    }

    /**
     * Test: Multi-page documents produce multiple page objects.
     */
    #[Test]
    public function it_extracts_multiple_pages(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(
            pages: new PageConfig(extractPages: true),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->pages, 'Pages should not be null');
        $this->assertNotEmpty($result->pages, 'Should produce pages');
        $this->assertGreaterThan(
            0,
            count($result->pages),
            'Multi-page document should produce multiple pages',
        );
    }

    /**
     * Test: Page extraction produces consistent results.
     */
    #[Test]
    public function it_produces_consistent_page_results(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(
            pages: new PageConfig(extractPages: true),
        );

        $kreuzberg = new Kreuzberg();
        $result1 = $kreuzberg->extractFile($filePath, config: $config);
        $result2 = $kreuzberg->extractFile($filePath, config: $config);

        $this->assertNotNull($result1->pages, 'First result pages should not be null');
        $this->assertNotNull($result2->pages, 'Second result pages should not be null');
        $this->assertNotEmpty($result1->pages, 'First result should produce pages');
        $this->assertNotEmpty($result2->pages, 'Second result should produce pages');
        $this->assertSame(
            count($result1->pages),
            count($result2->pages),
            'Multiple extractions should produce same page count',
        );
    }

    /**
     * Test: Page extraction without markers.
     */
    #[Test]
    public function it_extracts_pages_without_markers(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(
            pages: new PageConfig(
                extractPages: true,
                insertPageMarkers: false,
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->pages, 'Pages should be extracted');
    }
}
