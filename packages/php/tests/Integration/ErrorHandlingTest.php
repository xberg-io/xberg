<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Integration;

use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\EmbeddingConfig;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Kreuzberg;
use Kreuzberg\Tests\TestCase;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;

/**
 * Integration tests for error handling and edge cases.
 *
 * Comprehensive behavior-driven tests for error conditions, invalid configs,
 * malformed documents, and concurrent error states across extraction modes.
 * Tests exception types, readonly validation, and error message clarity.
 */
#[CoversClass(Kreuzberg::class)]
#[CoversClass(KreuzbergException::class)]
#[Group('integration')]
#[Group('errors')]
#[RequiresPhpExtension('kreuzberg-php')]
final class ErrorHandlingTest extends TestCase
{
    private string $testDocumentsPath;

    protected function setUp(): void
    {
        if (!extension_loaded('kreuzberg-php')) {
            $this->markTestSkipped('Kreuzberg extension is not loaded');
        }

        $this->testDocumentsPath = dirname(__DIR__, 4) . '/test_documents';
    }

    /**
     * Test 1: Invalid config handling - negative chunk size validation.
     */
    #[Test]
    public function it_throws_error_for_invalid_chunk_config(): void
    {
        $this->expectException(KreuzbergException::class);

        try {
            $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
            if (!file_exists($filePath)) {
                $this->markTestSkipped("Test file not found: {$filePath}");
            }

            // Invalid negative maxChunkSize should be rejected
            $config = new ExtractionConfig(
                chunking: new ChunkingConfig(maxChars: -100),
            );

            $kreuzberg = new Kreuzberg($config);
            $kreuzberg->extractFile($filePath);
        } catch (KreuzbergException $e) {
            // Validate error message contains meaningful information
            $message = $e->getMessage();
            $this->assertNotEmpty($message, 'Error message should not be empty');
            $this->assertTrue(
                stripos($message, 'negative') !== false ||
                stripos($message, 'maxChunkSize') !== false ||
                stripos($message, 'positive') !== false ||
                stripos($message, 'Extraction') !== false,
                "Error should mention constraint violation: {$message}",
            );
            throw $e;
        }
    }

    /**
     * Test 2: File not found error handling.
     */
    #[Test]
    public function it_throws_error_for_nonexistent_file(): void
    {
        $this->expectException(KreuzbergException::class);

        try {
            $kreuzberg = new Kreuzberg();
            $kreuzberg->extractFile('/this/path/definitely/does/not/exist.pdf');
            $this->fail('Expected KreuzbergException to be thrown');
        } catch (KreuzbergException $e) {
            $message = $e->getMessage();
            $this->assertNotEmpty($message, 'Error message should not be empty');
            $this->assertTrue(
                stripos($message, 'file') !== false ||
                stripos($message, 'not found') !== false ||
                stripos($message, 'exist') !== false,
                "Error should indicate file problem: {$message}",
            );
            throw $e;
        }
    }

    /**
     * Test 3: Corrupted/malformed document handling.
     */
    #[Test]
    public function it_throws_error_for_corrupted_pdf_document(): void
    {
        $this->expectException(KreuzbergException::class);

        $tmpFile = tempnam(sys_get_temp_dir(), 'krz_corrupted_');
        if ($tmpFile === false) {
            $this->markTestSkipped('Unable to create temporary file');
        }

        try {
            // Write corrupted PDF-like content
            file_put_contents($tmpFile, 'This is definitely not a valid PDF content');

            $kreuzberg = new Kreuzberg();
            $kreuzberg->extractFile($tmpFile);
        } finally {
            @unlink($tmpFile);
        }
    }

    /**
     * Test 4: Invalid MIME type handling.
     */
    #[Test]
    public function it_throws_error_for_invalid_mime_type(): void
    {
        $this->expectException(KreuzbergException::class);

        $tmpFile = tempnam(sys_get_temp_dir(), 'krz_test_');
        if ($tmpFile === false) {
            $this->markTestSkipped('Unable to create temporary file');
        }

        try {
            file_put_contents($tmpFile, 'test content');
            $bytes = file_get_contents($tmpFile);
            if ($bytes === false) {
                $this->fail('Unable to read temporary file');
            }

            $kreuzberg = new Kreuzberg();
            // Use a completely invalid MIME type
            $kreuzberg->extractBytes($bytes, 'invalid/totally-wrong-mime-type');
        } finally {
            @unlink($tmpFile);
        }
    }

    /**
     * Test 5: Permission denied error handling.
     */
    #[Test]
    public function it_throws_error_for_permission_denied_file(): void
    {
        $tmpFile = tempnam(sys_get_temp_dir(), 'krz_perm_');
        if ($tmpFile === false) {
            $this->markTestSkipped('Unable to create temporary file');
        }

        file_put_contents($tmpFile, 'test');

        try {
            // Remove read permission
            chmod($tmpFile, 0o000);

            $this->expectException(KreuzbergException::class);

            $kreuzberg = new Kreuzberg();
            $kreuzberg->extractFile($tmpFile);
        } finally {
            // Restore permission for cleanup
            chmod($tmpFile, 0o644);
            @unlink($tmpFile);
        }
    }

    /**
     * Test 6: Malformed document with valid extension handling.
     */
    #[Test]
    public function it_throws_error_for_mismatched_mime_type(): void
    {
        $this->expectException(KreuzbergException::class);

        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $bytes = file_get_contents($filePath);
        if ($bytes === false) {
            $this->fail('Unable to read test file');
        }

        $kreuzberg = new Kreuzberg();
        // Send PDF bytes but claim it's Excel format
        $kreuzberg->extractBytes($bytes, 'application/vnd.ms-excel');
    }

    /**
     * Test 7: Out-of-memory pattern simulation via configuration.
     */
    #[Test]
    public function it_handles_excessive_chunk_overlap_config(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        // Create config with excessive chunk overlap (>100% of chunk size)
        $config = new ExtractionConfig(
            chunking: new ChunkingConfig(
                maxChars: 100,
                maxOverlap: 150, // Overlap larger than chunk size
            ),
        );

        try {
            $kreuzberg = new Kreuzberg($config);
            // This may throw or handle gracefully - either is acceptable
            $kreuzberg->extractFile($filePath);
        } catch (KreuzbergException $e) {
            // Expected: invalid overlap configuration
            $this->assertStringContainsString('Extraction', $e->getMessage());
        }
    }

    /**
     * Test 8: Concurrent error states - batch with mixed valid/invalid files.
     */
    #[Test]
    public function it_throws_error_for_batch_with_invalid_files(): void
    {
        $this->expectException(KreuzbergException::class);

        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $files = [
            $filePath,
            '/nonexistent/invalid/path.pdf',
        ];

        $kreuzberg = new Kreuzberg();
        // Should fail when processing the invalid file in batch
        $kreuzberg->batchExtractFiles($files);
    }

    /**
     * Test 9: Batch bytes with mismatched count.
     */
    #[Test]
    public function it_throws_error_for_mismatched_batch_data(): void
    {
        $this->expectException(KreuzbergException::class);

        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $bytes = file_get_contents($filePath);
        if ($bytes === false) {
            $this->fail('Unable to read test file');
        }

        $dataList = [
            $bytes,
            $bytes,
        ];

        // Only provide one MIME type for two data items
        $mimeTypes = ['application/pdf'];

        $kreuzberg = new Kreuzberg();
        // Should fail due to array length mismatch
        $kreuzberg->batchExtractBytes($dataList, $mimeTypes);
    }

    /**
     * Test 10: Timeout behavior - long-running extraction with restrictive config.
     */
    #[Test]
    public function it_handles_extraction_with_restrictive_embedding_config(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        // Config with tiny batch size to test resource handling
        $config = new ExtractionConfig(
            embedding: new EmbeddingConfig(
                model: 'all-MiniLM-L6-v2',
                batchSize: 1, // Very small batch size
            ),
        );

        try {
            $kreuzberg = new Kreuzberg($config);
            $result = $kreuzberg->extractFile($filePath);

            // If successful, verify we got results
            $this->assertIsObject($result);
        } catch (KreuzbergException $e) {
            // Either success or exception is acceptable for this edge case
            $this->assertStringContainsString(
                'Extraction',
                $e->getMessage(),
            );
        }
    }

    /**
     * Test readonly config enforcement - verify configs cannot be modified after creation.
     */
    #[Test]
    public function it_enforces_readonly_chunking_config(): void
    {
        $config = new ChunkingConfig(
            maxChars: 512,
            maxOverlap: 50,
        );

        $this->assertEquals(512, $config->maxChars);
        $this->assertEquals(50, $config->maxOverlap);

        // Attempting to modify should fail due to readonly property
        $this->expectError();
        // @phpstan-ignore-next-line
        $config->maxChars = 1024; // This should error
    }

    /**
     * Test readonly extraction config enforcement.
     */
    #[Test]
    public function it_enforces_readonly_extraction_config(): void
    {
        $config = new ExtractionConfig(
            extractImages: true,
            extractTables: false,
        );

        $this->assertTrue($config->extractImages);
        $this->assertFalse($config->extractTables);

        // Attempting to modify should fail due to readonly property
        $this->expectError();
        // @phpstan-ignore-next-line
        $config->extractImages = false; // This should error
    }

    /**
     * Test exception type and code for validation errors.
     */
    #[Test]
    public function it_creates_validation_exception_with_proper_code(): void
    {
        $exception = KreuzbergException::validation('Invalid parameter value');

        $this->assertInstanceOf(KreuzbergException::class, $exception);
        $this->assertSame(1, $exception->getCode());
        $this->assertStringContainsString('Validation error', $exception->getMessage());
    }

    /**
     * Test exception type and code for parsing errors.
     */
    #[Test]
    public function it_creates_parsing_exception_with_proper_code(): void
    {
        $exception = KreuzbergException::parsing('Failed to parse document');

        $this->assertInstanceOf(KreuzbergException::class, $exception);
        $this->assertSame(2, $exception->getCode());
        $this->assertStringContainsString('Parsing error', $exception->getMessage());
    }

    /**
     * Test exception type and code for OCR errors.
     */
    #[Test]
    public function it_creates_ocr_exception_with_proper_code(): void
    {
        $exception = KreuzbergException::ocr('OCR backend unavailable');

        $this->assertInstanceOf(KreuzbergException::class, $exception);
        $this->assertSame(3, $exception->getCode());
        $this->assertStringContainsString('OCR error', $exception->getMessage());
    }

    /**
     * Test exception type and code for I/O errors.
     */
    #[Test]
    public function it_creates_io_exception_with_proper_code(): void
    {
        $exception = KreuzbergException::io('Failed to read file');

        $this->assertInstanceOf(KreuzbergException::class, $exception);
        $this->assertSame(5, $exception->getCode());
        $this->assertStringContainsString('I/O error', $exception->getMessage());
    }

    /**
     * Test exception type and code for unsupported format errors.
     */
    #[Test]
    public function it_creates_unsupported_format_exception_with_proper_code(): void
    {
        $exception = KreuzbergException::unsupportedFormat('TIFF not supported');

        $this->assertInstanceOf(KreuzbergException::class, $exception);
        $this->assertSame(7, $exception->getCode());
        $this->assertStringContainsString('Unsupported format', $exception->getMessage());
    }

    /**
     * Test empty bytes handling.
     */
    #[Test]
    public function it_throws_error_for_empty_bytes(): void
    {
        $this->expectException(KreuzbergException::class);

        $kreuzberg = new Kreuzberg();
        $kreuzberg->extractBytes('', 'application/pdf');
    }

    /**
     * Test empty file path handling.
     */
    #[Test]
    public function it_throws_error_for_empty_file_path(): void
    {
        $this->expectException(KreuzbergException::class);

        $kreuzberg = new Kreuzberg();
        $kreuzberg->extractFile('');
    }

    /**
     * Test very long file path handling.
     */
    #[Test]
    public function it_throws_error_for_extremely_long_file_path(): void
    {
        $this->expectException(KreuzbergException::class);

        $longPath = str_repeat('/nonexistent', 500) . '/file.pdf';

        $kreuzberg = new Kreuzberg();
        $kreuzberg->extractFile($longPath);
    }

    /**
     * Test special characters in file path.
     */
    #[Test]
    public function it_throws_error_for_special_chars_in_file_path(): void
    {
        $this->expectException(KreuzbergException::class);

        $specialPath = '/tmp/file with spaces & special chars!@#$%^.pdf';

        $kreuzberg = new Kreuzberg();
        $kreuzberg->extractFile($specialPath);
    }

    /**
     * Test null bytes in file path.
     */
    #[Test]
    public function it_throws_error_for_null_bytes_in_path(): void
    {
        $pathWithNull = "/tmp/file\0.pdf";

        $this->expectException(KreuzbergException::class);

        $kreuzberg = new Kreuzberg();
        $kreuzberg->extractFile($pathWithNull);
    }

    /**
     * Test directory instead of file.
     */
    #[Test]
    public function it_throws_error_when_given_directory_instead_of_file(): void
    {
        $this->expectException(KreuzbergException::class);

        $kreuzberg = new Kreuzberg();
        $kreuzberg->extractFile($this->testDocumentsPath);
    }

    /**
     * Test meaningful error messages.
     */
    #[Test]
    public function it_provides_meaningful_error_messages(): void
    {
        try {
            $kreuzberg = new Kreuzberg();
            $kreuzberg->extractFile('/nonexistent/file.pdf');

            $this->fail('Expected KreuzbergException to be thrown');
        } catch (KreuzbergException $e) {
            $message = $e->getMessage();
            $this->assertNotEmpty($message);
            $this->assertIsString($message);
            // Message should contain useful context
            $this->assertTrue(
                strlen($message) > 5,
                'Error message should be descriptive',
            );
        }
    }

    /**
     * Test OCR config validation.
     */
    #[Test]
    public function it_validates_ocr_configuration(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        // Create OCR config with empty backend
        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: '',
                language: 'eng',
            ),
        );

        try {
            $kreuzberg = new Kreuzberg($config);
            $kreuzberg->extractFile($filePath);
        } catch (KreuzbergException $e) {
            // Empty backend may be caught and reported
            $this->assertNotEmpty($e->getMessage());
        }
    }

    /**
     * Test embedding config validation.
     */
    #[Test]
    public function it_validates_embedding_configuration(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        // Create embedding config with negative batch size (edge case)
        $config = new ExtractionConfig(
            embedding: new EmbeddingConfig(
                model: 'all-MiniLM-L6-v2',
                batchSize: -1, // Invalid
            ),
        );

        try {
            $kreuzberg = new Kreuzberg($config);
            $kreuzberg->extractFile($filePath);
        } catch (KreuzbergException $e) {
            // Negative batch size should be caught
            $this->assertNotEmpty($e->getMessage());
        }
    }

    /**
     * Test invalid JSON in config file.
     */
    #[Test]
    public function it_throws_error_for_invalid_json_config(): void
    {
        $tmpFile = tempnam(sys_get_temp_dir(), 'krz_json_');
        if ($tmpFile === false) {
            $this->markTestSkipped('Unable to create temporary file');
        }

        try {
            file_put_contents($tmpFile, '{invalid json content}');

            $this->expectException(\InvalidArgumentException::class);
            ExtractionConfig::fromFile($tmpFile);
        } finally {
            @unlink($tmpFile);
        }
    }

    /**
     * Test nonexistent config file.
     */
    #[Test]
    public function it_throws_error_for_missing_config_file(): void
    {
        $this->expectException(\InvalidArgumentException::class);
        $this->expectExceptionMessage('not found');

        ExtractionConfig::fromFile('/nonexistent/config.json');
    }
}
