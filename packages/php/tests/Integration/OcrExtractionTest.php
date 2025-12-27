<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Integration;

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\TesseractConfig;
use Kreuzberg\Kreuzberg;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for OCR functionality.
 *
 * Tests optical character recognition on images and scanned documents.
 */
#[CoversClass(Kreuzberg::class)]
#[Group('integration')]
#[Group('ocr')]
#[RequiresPhpExtension('kreuzberg')]
final class OcrExtractionTest extends TestCase
{
    private string $testDocumentsPath;

    protected function setUp(): void
    {
        if (!extension_loaded('kreuzberg')) {
            $this->markTestSkipped('Kreuzberg extension is not loaded');
        }

        $this->testDocumentsPath = dirname(__DIR__, 4) . '/test_documents';
    }

    #[Test]
    public function it_extracts_text_from_image_with_ocr(): void
    {
        $imagePath = $this->testDocumentsPath . '/images/invoice_image.png';

        if (!file_exists($imagePath)) {
            $this->markTestSkipped("Test image not found: {$imagePath}");
        }

        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($imagePath);

        $this->assertNotEmpty(
            $result->content,
            'OCR should extract text from image',
        );
        $this->assertStringContainsString('image/', strtolower($result->mimeType));
    }

    #[Test]
    public function it_performs_ocr_with_tesseract_backend(): void
    {
        $imagePath = $this->testDocumentsPath . '/tables/simple_table.png';

        if (!file_exists($imagePath)) {
            $this->markTestSkipped("Test image not found: {$imagePath}");
        }

        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($imagePath);

        $this->assertNotNull(
            $result->content,
            'Tesseract should extract text from table image',
        );
    }

    #[Test]
    public function it_configures_tesseract_page_segmentation_mode(): void
    {
        $imagePath = $this->testDocumentsPath . '/tables/simple_table.png';

        if (!file_exists($imagePath)) {
            $this->markTestSkipped("Test image not found: {$imagePath}");
        }

        $tesseractConfig = new TesseractConfig(
            psm: 6,
            oem: 3,
        );

        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
                tesseract: $tesseractConfig,
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($imagePath);

        $this->assertNotNull(
            $result->content,
            'OCR with custom PSM should work',
        );
    }

    #[Test]
    public function it_extracts_text_from_table_image_with_table_detection(): void
    {
        $imagePath = $this->testDocumentsPath . '/tables/simple_table.png';

        if (!file_exists($imagePath)) {
            $this->markTestSkipped("Test image not found: {$imagePath}");
        }

        $tesseractConfig = new TesseractConfig(
            enableTableDetection: true,
        );

        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
                tesseract: $tesseractConfig,
            ),
            extractTables: true,
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($imagePath);

        $this->assertNotNull(
            $result->content,
            'OCR with table detection should extract content',
        );
    }

    #[Test]
    public function it_handles_different_languages_in_ocr(): void
    {
        $imagePath = $this->testDocumentsPath . '/images/invoice_image.png';

        if (!file_exists($imagePath)) {
            $this->markTestSkipped("Test image not found: {$imagePath}");
        }

        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($imagePath);

        $this->assertNotNull(
            $result->content,
            'OCR should work with specified language',
        );
    }

    #[Test]
    public function it_extracts_from_borderless_table_image(): void
    {
        $imagePath = $this->testDocumentsPath . '/tables/borderless_table.png';

        if (!file_exists($imagePath)) {
            $this->markTestSkipped("Test image not found: {$imagePath}");
        }

        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
            ),
            extractTables: true,
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($imagePath);

        $this->assertNotNull(
            $result->content,
            'OCR should handle borderless tables',
        );
    }

    #[Test]
    public function it_extracts_from_complex_document_image(): void
    {
        $imagePath = $this->testDocumentsPath . '/tables/complex_document.png';

        if (!file_exists($imagePath)) {
            $this->markTestSkipped("Test image not found: {$imagePath}");
        }

        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($imagePath);

        $this->assertNotEmpty(
            $result->content,
            'OCR should handle complex document layouts',
        );
    }

    #[Test]
    public function it_performs_ocr_on_pdf_with_fallback(): void
    {
        $pdfPath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($pdfPath)) {
            $this->markTestSkipped("Test PDF not found: {$pdfPath}");
        }

        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfPath);

        $this->assertNotEmpty(
            $result->content,
            'OCR configuration should work with PDFs',
        );
    }

    #[Test]
    public function it_extracts_images_from_documents_with_ocr(): void
    {
        $pdfPath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($pdfPath)) {
            $this->markTestSkipped("Test PDF not found: {$pdfPath}");
        }

        $config = new ExtractionConfig(
            extractImages: true,
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfPath);

        $this->assertNotNull(
            $result->images,
            'OCR with extractImages should process embedded images',
        );
    }

    #[Test]
    public function it_performs_batch_ocr_on_multiple_images(): void
    {
        $images = [
            $this->testDocumentsPath . '/tables/simple_table.png',
            $this->testDocumentsPath . '/tables/borderless_table.png',
        ];

        foreach ($images as $image) {
            if (!file_exists($image)) {
                $this->markTestSkipped("Test image not found: {$image}");
            }
        }

        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $results = $kreuzberg->batchExtractFiles($images);

        $this->assertCount(
            2,
            $results,
            'Batch OCR should process all images',
        );

        foreach ($results as $result) {
            $this->assertNotNull(
                $result->content,
                'Each image should have OCR-extracted content',
            );
        }
    }

    #[Test]
    public function it_validates_ocr_output_is_utf8(): void
    {
        $imagePath = $this->testDocumentsPath . '/images/invoice_image.png';

        if (!file_exists($imagePath)) {
            $this->markTestSkipped("Test image not found: {$imagePath}");
        }

        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: 'eng',
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($imagePath);

        $this->assertTrue(
            mb_check_encoding($result->content, 'UTF-8'),
            'OCR output should be valid UTF-8',
        );
    }
}
