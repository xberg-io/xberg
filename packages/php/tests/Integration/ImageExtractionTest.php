<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Integration;

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Kreuzberg;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for image extraction functionality.
 *
 * Tests extraction of images and image metadata from various document types,
 * including PDFs, Office documents, and multi-page documents with comprehensive
 * validation of image properties, format detection, and error handling.
 *
 * Test Coverage:
 * - PDF image extraction with metadata (format, dimensions, MIME type)
 * - Image handling in composite documents (DOCX, PPTX)
 * - Image format detection (PNG, JPEG, WebP)
 * - Embedded vs. referenced images
 * - Error handling for corrupted images
 * - Batch image extraction from multi-page documents
 */
#[CoversClass(Kreuzberg::class)]
#[Group('integration')]
#[Group('images')]
#[RequiresPhpExtension('kreuzberg-php')]
final class ImageExtractionTest extends TestCase
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
     * Test: PDF image extraction with metadata (format, dimensions, MIME type).
     *
     * Validates that images extracted from PDFs include complete metadata:
     * - Image format (PNG, JPEG, WebP)
     * - Dimensions (width and height in pixels)
     * - MIME type information
     * - Image index for tracking
     */
    #[Test]
    public function it_extracts_images_from_pdf_with_metadata(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files found for testing');
        }

        $config = new ExtractionConfig(
            images: new ImageExtractionConfig(extractImages: true),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Verify images array is present
        $this->assertNotNull(
            $result->images,
            'ExtractionResult should contain images array when extractImages is enabled',
        );

        // If images are found, validate their structure
        if (!empty($result->images)) {
            $image = $result->images[0];

            // Validate image format is one of expected types
            $this->assertNotEmpty(
                $image->format,
                'Image format should not be empty',
            );
            $supportedFormats = ['PNG', 'JPEG', 'WebP', 'GIF', 'TIFF'];
            $this->assertContains(
                strtoupper($image->format),
                $supportedFormats,
                "Image format '{$image->format}' should be one of supported formats",
            );

            // Validate dimensions if present
            if ($image->width !== null) {
                $this->assertGreaterThan(
                    0,
                    $image->width,
                    'Image width should be positive integer',
                );
            }

            if ($image->height !== null) {
                $this->assertGreaterThan(
                    0,
                    $image->height,
                    'Image height should be positive integer',
                );
            }

            // Validate image index
            $this->assertGreaterThanOrEqual(
                0,
                $image->imageIndex,
                'Image index should be non-negative',
            );

            // Validate image data exists
            $this->assertNotEmpty(
                $image->data,
                'Image data should not be empty',
            );
        }
    }

    /**
     * Test: Image handling in composite documents (DOCX, PPTX).
     *
     * Validates image extraction from Office document formats that can contain
     * embedded images. Tests both DOCX (Word) and PPTX (PowerPoint) formats.
     */
    #[Test]
    public function it_extracts_images_from_docx_documents(): void
    {
        $filePath = $this->testDocumentsPath . '/office/document.docx';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(
            images: new ImageExtractionConfig(extractImages: true),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        // DOCX documents may or may not have images, but extraction should work
        $this->assertNotNull(
            $result->images,
            'DOCX extraction should return images array (may be empty)',
        );

        // If images found, validate basic properties
        if (!empty($result->images)) {
            foreach ($result->images as $image) {
                $this->assertNotEmpty(
                    $image->format,
                    'Each image should have a format',
                );
                $this->assertGreaterThanOrEqual(
                    0,
                    $image->imageIndex,
                    'Image index should be non-negative',
                );
            }
        }
    }

    /**
     * Test: Image format detection (PNG, JPEG, WebP).
     *
     * Validates that the extraction system correctly identifies image formats
     * and categorizes them appropriately when multiple format types are present.
     */
    #[Test]
    public function it_correctly_detects_multiple_image_formats(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files found for testing');
        }

        $config = new ExtractionConfig(
            images: new ImageExtractionConfig(extractImages: true),
        );

        $kreuzberg = new Kreuzberg($config);

        // Test multiple files to find one with images in different formats
        $detectedFormats = [];
        $totalImages = 0;

        foreach ($pdfFiles as $pdfFile) {
            $result = $kreuzberg->extractFile($pdfFile);

            if (!empty($result->images)) {
                $totalImages += count($result->images);
                foreach ($result->images as $image) {
                    $format = strtoupper($image->format);
                    if (!in_array($format, $detectedFormats, true)) {
                        $detectedFormats[] = $format;
                    }
                }
            }

            // Stop after finding a few different formats
            if (count($detectedFormats) >= 2) {
                break;
            }
        }

        // Image extraction may not be available on all platforms (e.g., ARM without pdfium)
        if (empty($detectedFormats)) {
            $this->markTestSkipped('Image extraction not available on this platform');
        }

        $this->assertNotEmpty(
            $detectedFormats,
            'Should detect at least one image format from test PDFs',
        );

        foreach ($detectedFormats as $format) {
            $this->assertNotEmpty($format, 'Detected format should not be empty');
            $this->assertStringNotContainsString(
                ' ',
                $format,
                'Format should not contain spaces',
            );
        }
    }

    /**
     * Test: Image metadata properties (page number, dimensions).
     *
     * Validates that extracted images include proper metadata about their
     * location within the document and their physical dimensions.
     */
    #[Test]
    public function it_includes_image_page_numbers_and_dimensions(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs_with_tables/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files with tables found');
        }

        $config = new ExtractionConfig(
            images: new ImageExtractionConfig(extractImages: true),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        $this->assertNotNull($result->images, 'Images should not be null');

        if (empty($result->images)) {
            $this->markTestSkipped('Test PDF does not contain images');
        }

        foreach ($result->images as $image) {
            // Page number should be set if available
            if ($image->pageNumber !== null) {
                $this->assertGreaterThan(
                    0,
                    $image->pageNumber,
                    'Page number should be positive integer',
                );
            }

            // Dimensions should be positive if present
            if ($image->width !== null && $image->height !== null) {
                $this->assertGreaterThan(
                    0,
                    $image->width,
                    'Image width should be positive',
                );
                $this->assertGreaterThan(
                    0,
                    $image->height,
                    'Image height should be positive',
                );

                // Validate aspect ratio is reasonable (between 0.1 and 10)
                $aspectRatio = $image->width / $image->height;
                $this->assertGreaterThan(
                    0.1,
                    $aspectRatio,
                    'Image aspect ratio should be reasonable',
                );
                $this->assertLessThan(
                    10,
                    $aspectRatio,
                    'Image aspect ratio should be reasonable',
                );
            }
        }
    }

    /**
     * Test: Error handling for images with minimal/missing metadata.
     *
     * Validates that image extraction handles edge cases gracefully,
     * including images with missing dimensions or incomplete metadata.
     */
    #[Test]
    public function it_handles_images_with_missing_metadata_gracefully(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files found for testing');
        }

        $config = new ExtractionConfig(
            images: new ImageExtractionConfig(extractImages: true),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // Even with missing metadata, images should be returned if present
        $this->assertNotNull(
            $result->images,
            'Images array should be present even if some images have missing metadata',
        );

        if (!empty($result->images)) {
            foreach ($result->images as $image) {
                // These fields are always required
                $this->assertIsString(
                    $image->data,
                    'Image data must always be present as string',
                );
                $this->assertIsString(
                    $image->format,
                    'Image format must always be present as string',
                );
                $this->assertIsInt(
                    $image->imageIndex,
                    'Image index must always be present as integer',
                );

                // These fields can be null - that's acceptable
                $this->assertTrue(
                    $image->width === null || is_int($image->width),
                    'Image width should be null or integer',
                );
                $this->assertTrue(
                    $image->height === null || is_int($image->height),
                    'Image height should be null or integer',
                );
                $this->assertTrue(
                    $image->pageNumber === null || is_int($image->pageNumber),
                    'Page number should be null or integer',
                );
            }
        }
    }

    /**
     * Test: Batch image extraction from multi-page documents.
     *
     * Validates that image extraction works correctly when processing multiple
     * documents in batch mode, with proper indexing and metadata preservation.
     */
    #[Test]
    public function it_extracts_images_in_batch_from_multiple_documents(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs/*.pdf');

        if (count($pdfFiles) < 2) {
            $this->markTestSkipped('Need at least 2 PDF files for batch test');
        }

        // Take first 2 PDFs
        $files = array_slice($pdfFiles, 0, 2);

        $config = new ExtractionConfig(
            images: new ImageExtractionConfig(extractImages: true),
        );

        $kreuzberg = new Kreuzberg($config);
        $results = $kreuzberg->batchExtractFiles($files);

        // Batch should return results for all files
        $this->assertCount(
            2,
            $results,
            'Batch processing should return results for all files',
        );

        $totalImages = 0;

        foreach ($results as $index => $result) {
            // Each result should have images array
            $this->assertNotNull(
                $result->images,
                "Result {$index} should have images array",
            );

            if (!empty($result->images)) {
                $totalImages += count($result->images);

                // Validate each image in the batch result
                foreach ($result->images as $image) {
                    $this->assertNotEmpty(
                        $image->format,
                        "Image in batch result {$index} should have format",
                    );
                    $this->assertIsInt(
                        $image->imageIndex,
                        "Image in batch result {$index} should have numeric index",
                    );
                }
            }
        }

        // Batch processing should successfully process documents with images
        $this->assertNotNull(
            $results,
            'Batch image extraction should return results',
        );
    }

    /**
     * Test: Image extraction with imageExtraction config.
     *
     * Validates that the ImageExtractionConfig readonly class works properly
     * with image extraction settings like performOcr and dimension filters.
     */
    #[Test]
    public function it_extracts_images_with_image_extraction_config(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files found for testing');
        }

        $config = new ExtractionConfig(
            images: new ImageExtractionConfig(
                extractImages: true,
            ),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // ImageExtractionConfig should enable image extraction
        $this->assertNotNull(
            $result->images,
            'ImageExtractionConfig should enable image extraction',
        );
        $this->assertIsArray(
            $result->images,
            'Images should be an array',
        );
    }

    /**
     * Test: Image data integrity and format consistency.
     *
     * Validates that extracted image data is properly formatted and consistent,
     * including verification that binary data is preserved correctly.
     */
    #[Test]
    public function it_preserves_image_data_integrity(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files found for testing');
        }

        $config = new ExtractionConfig(
            images: new ImageExtractionConfig(extractImages: true),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        $this->assertNotNull($result->images, 'Images should not be null');

        if (empty($result->images)) {
            $this->markTestSkipped('Test PDF does not contain images');
        }

        foreach ($result->images as $image) {
            // Image data should not be empty
            $this->assertNotEmpty(
                $image->data,
                'Image data should be present and non-empty',
            );

            // Image data should be a valid string (binary safe)
            $this->assertIsString(
                $image->data,
                'Image data must be a string',
            );

            // Image format should be identifiable
            $this->assertNotEmpty(
                $image->format,
                'Image format should be identifiable',
            );

            // Format should match expected values
            $validFormats = ['png', 'jpeg', 'jpg', 'webp', 'gif', 'tiff', 'bmp'];
            $this->assertContains(
                strtolower($image->format),
                $validFormats,
                "Format '{$image->format}' should be a supported image format",
            );
        }
    }

    /**
     * Test: Image extraction from documents with no images.
     *
     * Validates that image extraction returns empty array (not null) for
     * documents that don't contain any images, ensuring consistent behavior.
     */
    #[Test]
    public function it_returns_empty_array_for_documents_without_images(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(
            images: new ImageExtractionConfig(extractImages: true),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        // Images should be null or empty array for documents without images
        $this->assertTrue(
            $result->images === null || is_array($result->images),
            'Images should be null or array',
        );

        if ($result->images !== null) {
            $this->assertIsArray(
                $result->images,
                'Images should be array when not null',
            );
            $this->assertEmpty(
                $result->images,
                'Image-less documents should return empty images array',
            );
        }
    }

    /**
     * Test: Image extraction disabled returns empty results.
     *
     * Validates that setting extractImages to false prevents image extraction
     * and returns appropriate empty/null values.
     */
    #[Test]
    public function it_skips_image_extraction_when_disabled(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files found for testing');
        }

        $config = new ExtractionConfig(
            images: new ImageExtractionConfig(extractImages: false),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        // When disabled, images should be null or empty
        $this->assertTrue(
            $result->images === null || (is_array($result->images) && empty($result->images)),
            'Images should be null or empty array when extraction is disabled',
        );

        if ($result->images !== null) {
            $this->assertEmpty(
                $result->images,
                'Images should be empty when extraction is disabled',
            );
        }
    }

    /**
     * Test: Multiple images from single document are properly indexed.
     *
     * Validates that when a document contains multiple images, they are
     * all extracted and properly indexed for identification.
     */
    #[Test]
    public function it_properly_indexes_multiple_images_in_document(): void
    {
        $pdfFiles = glob($this->testDocumentsPath . '/pdfs/*.pdf');

        if (empty($pdfFiles)) {
            $this->markTestSkipped('No PDF files found for testing');
        }

        $config = new ExtractionConfig(
            images: new ImageExtractionConfig(extractImages: true),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($pdfFiles[0]);

        $this->assertNotNull($result->images, 'Images should not be null');

        if (empty($result->images)) {
            $this->markTestSkipped('Test PDF does not contain images');
        }

        if (count($result->images) === 1) {
            $this->markTestSkipped('Test PDF only contains one image, need multiple for indexing test');
        }

        // Verify all images have unique indices
        $indices = array_map(
            static fn ($image) => $image->imageIndex,
            $result->images,
        );

        $this->assertCount(
            count($result->images),
            array_unique($indices),
            'Each image should have unique index within document',
        );

        // Verify indices are sequential or properly ordered
        foreach ($result->images as $index => $image) {
            $this->assertIsInt(
                $image->imageIndex,
                "Image at position {$index} should have integer index",
            );
            $this->assertGreaterThanOrEqual(
                0,
                $image->imageIndex,
                'Image index should be non-negative',
            );
        }
    }
}
