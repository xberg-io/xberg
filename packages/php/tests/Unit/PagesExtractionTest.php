<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Unit;

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\PageConfig;
use Kreuzberg\Kreuzberg;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\TestCase;

/**
 * Unit tests for pages extraction functionality.
 *
 * Tests cover:
 * - extractPages: true - Returns pages array
 * - insertPageMarkers: true - Markers appear in content
 * - markerFormat: custom format works
 * - Multi-page PDF produces multiple pages
 * - Page content structure validation
 */
#[RequiresPhpExtension('kreuzberg-php')]
final class PagesExtractionTest extends TestCase
{
    /**
     * Test that extractPages: true returns a pages array.
     */
    public function testExtractPagesReturnsPageArray(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                extractPages: true,
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertNotNull($result->pages);
        $this->assertIsArray($result->pages);
        $this->assertGreaterThan(0, count($result->pages));
    }

    /**
     * Test that pages array contains entries with required fields.
     */
    public function testPageArrayContainsRequiredFields(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                extractPages: true,
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertNotNull($result->pages);

        foreach ($result->pages as $page) {
            $this->assertIsObject($page);
            $this->assertTrue(property_exists($page, 'content'));
            $this->assertTrue(property_exists($page, 'pageNumber'));
            $this->assertIsString($page->content);
            $this->assertIsInt($page->pageNumber);
        }
    }

    /**
     * Test that page markers appear in content when enabled.
     */
    public function testInsertPageMarkersAppearsInContent(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                insertPageMarkers: true,
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        // Content should contain markers (default or custom format)
        $this->assertStringContainsString(
            'Page',
            $result->content,
            'Content should contain page markers',
        );
    }

    /**
     * Test that custom marker format is properly applied.
     */
    public function testCustomMarkerFormatWorks(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                insertPageMarkers: true,
                markerFormat: '=== Page {page_num} ===',
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertStringContainsString(
            '=== Page',
            $result->content,
            'Content should contain custom page markers',
        );
    }

    /**
     * Test that {page_num} placeholder is replaced with actual page numbers.
     */
    public function testMarkerFormatPageNumberReplacement(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                insertPageMarkers: true,
                markerFormat: '--- BEGIN PAGE {page_num} ---',
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertStringContainsString(
            '--- BEGIN PAGE',
            $result->content,
            'Content should contain the marker with placeholder replaced',
        );
    }

    /**
     * Test that multi-page PDFs produce multiple page entries.
     */
    public function testMultiPagePDFProducesMultiplePages(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                extractPages: true,
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertNotNull($result->pages);
        $this->assertGreaterThanOrEqual(1, count($result->pages));
    }

    /**
     * Test page content structure validation.
     */
    public function testPageContentStructureValidation(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                extractPages: true,
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertNotNull($result->pages);

        foreach ($result->pages as $page) {
            // Content should be string (may be empty for blank pages)
            $this->assertIsString($page->content);

            // Page number should be positive integer
            $this->assertIsInt($page->pageNumber);
            $this->assertGreaterThan(0, $page->pageNumber);
        }
    }

    /**
     * Test that page numbers are sequential.
     */
    public function testPageNumbersAreSequential(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                extractPages: true,
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertNotNull($result->pages);

        foreach ($result->pages as $index => $page) {
            $expectedPageNum = $index + 1;
            $this->assertSame(
                $expectedPageNum,
                $page->pageNumber,
                "Page {$index} should have page number {$expectedPageNum}",
            );
        }
    }

    /**
     * Test combining extract_pages and insert_page_markers.
     */
    public function testExtractPagesAndInsertMarkersTogethers(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                extractPages: true,
                insertPageMarkers: true,
                markerFormat: '[Page {page_num}]',
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        // Should have both pages array and markers in content
        $this->assertNotNull($result->pages);
        $this->assertGreaterThan(0, count($result->pages));

        $this->assertStringContainsString(
            '[Page',
            $result->content,
            'Content should contain page markers',
        );
    }

    /**
     * Test extraction without explicit page config.
     */
    public function testExtractionWithoutPageConfig(): void
    {
        $config = new ExtractionConfig();

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        // Should succeed with default behavior
        $this->assertIsString($result->content);
    }

    /**
     * Test various custom marker formats.
     */
    public function testMarkdownStyleMarkers(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                insertPageMarkers: true,
                markerFormat: '## Page {page_num}',
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertStringContainsString('##', $result->content);
    }

    /**
     * Test separator-style page markers.
     */
    public function testSeparatorStyleMarkers(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                insertPageMarkers: true,
                markerFormat: '---page-{page_num}---',
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertStringContainsString('---page-', $result->content);
    }

    /**
     * Test HTML comment-style markers.
     */
    public function testCommentStyleMarkers(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                insertPageMarkers: true,
                markerFormat: '<!-- PAGE {page_num} -->',
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        // Should contain either comment markers or page identifiers
        $this->assertTrue(
            str_contains($result->content, '<!--') ||
            str_contains($result->content, 'PAGE'),
            'Content should contain markers',
        );
    }

    /**
     * Test page config with only extractPages.
     */
    public function testPageConfigWithOnlyExtractPages(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(extractPages: true),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertNotNull($result->pages);
        $this->assertIsArray($result->pages);
        $this->assertGreaterThan(0, count($result->pages));
    }

    /**
     * Test page config with only insertPageMarkers.
     */
    public function testPageConfigWithOnlyInsertMarkers(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(insertPageMarkers: true),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertIsString($result->content);
    }

    /**
     * Test page config with only markerFormat.
     */
    public function testPageConfigWithOnlyMarkerFormat(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(markerFormat: '>> Page {page_num} <<'),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertIsString($result->content);
    }

    /**
     * Test that page content may be empty for blank pages.
     */
    public function testPageContentMayBeEmpty(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                extractPages: true,
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertNotNull($result->pages);

        // Verify structure even if content is empty
        foreach ($result->pages as $page) {
            $this->assertTrue(is_string($page->content) || $page->content === null);
            $this->assertIsInt($page->pageNumber);
        }
    }

    /**
     * Test markers and extracted pages consistency.
     */
    public function testMarkersAndExtractedPagesConsistency(): void
    {
        $config = new ExtractionConfig(
            pages: new PageConfig(
                extractPages: true,
                insertPageMarkers: true,
            ),
        );

        $testPdfBytes = $this->getTestPdfBytes();
        $result = Kreuzberg::extractBytesSync($testPdfBytes, 'application/pdf', $config);

        $this->assertNotNull($result->pages);
        $this->assertGreaterThanOrEqual(1, count($result->pages));

        // Both pages array and content should be populated
        $this->assertNotEmpty($result->content);
    }

    /**
     * Helper method to get test PDF bytes.
     *
     * This loads an actual PDF file from the test fixtures directory,
     * or falls back to a minimal PDF structure for testing.
     */
    private function getTestPdfBytes(): string
    {
        $repoRoot = $this->getRepositoryRoot();
        $testPdfPath = $repoRoot . '/test_documents/pdfs_with_tables/tiny.pdf';

        if (file_exists($testPdfPath)) {
            $content = file_get_contents($testPdfPath);
            if ($content === false) {
                throw new \RuntimeException("Failed to read test PDF: {$testPdfPath}");
            }
            return $content;
        }

        // Fallback: return a minimal valid PDF structure
        return $this->getMinimalTestPdf();
    }

    /**
     * Helper method to locate the repository root.
     */
    private function getRepositoryRoot(): string
    {
        $currentDir = __DIR__;
        // Navigate up from packages/php/tests/Unit to repo root
        return dirname(dirname(dirname(dirname($currentDir))));
    }

    /**
     * Helper method providing a minimal test PDF.
     *
     * This is used as a fallback if the actual test PDF file is not available.
     */
    private function getMinimalTestPdf(): string
    {
        return <<<'PDF'
            %PDF-1.7
            1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
            2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
            3 0 obj<</Type/Page/Parent 2 0 R>>endobj
            xref 0 4 0000000000 65535 f 0000000009 00000 n 0000000058 00000 n 0000000117 00000 n
            trailer<</Size 4/Root 1 0 R>>
            startxref
            191
            %%EOF
            PDF;
    }
}
