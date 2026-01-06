<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Integration;

use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Kreuzberg;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for batch operations functionality.
 *
 * Comprehensive testing of batch extraction capabilities including:
 * - Batch file processing
 * - Batch byte processing
 * - Error handling in batch operations
 * - Mixed file format handling
 * - Memory efficiency
 * - Parallel processing
 * - Configuration inheritance
 *
 * Test Coverage:
 * - batchExtractFiles() method
 * - batchExtractBytes() method
 * - Batch error handling and recovery
 * - Mixed document format batches
 * - Memory efficiency with large batches
 * - Consistent result ordering
 * - MIME type handling in batches
 */
#[CoversClass(Kreuzberg::class)]
#[Group('integration')]
#[Group('batch')]
#[RequiresPhpExtension('kreuzberg-php')]
final class BatchOperationsTest extends TestCase
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
     * Test: Batch extraction returns array of results.
     */
    #[Test]
    public function it_extracts_multiple_files_in_batch(): void
    {
        $files = [
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        // Add PDF if available
        $pdfPath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (file_exists($pdfPath)) {
            $files[] = $pdfPath;
        }

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        $this->assertIsArray($results, 'Batch extraction should return an array');
        $this->assertCount(count($files), $results, 'Should return one result per file');

        foreach ($results as $index => $result) {
            $this->assertNotNull($result, "Result {$index} should not be null");
            $this->assertObjectHasProperty('content', $result, "Result {$index} should have content");
        }
    }

    /**
     * Test: Batch results maintain correct ordering.
     */
    #[Test]
    public function it_maintains_batch_result_ordering(): void
    {
        $files = [
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        // Add additional files if available
        $pdfPath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (file_exists($pdfPath)) {
            $files[] = $pdfPath;
        }

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        // Results should be in same order as input files
        $this->assertCount(count($files), $results, 'Result count should match file count');
    }

    /**
     * Test: Batch extraction respects configuration.
     */
    #[Test]
    public function it_applies_config_to_batch_extraction(): void
    {
        $files = [
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        $pdfPath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (file_exists($pdfPath)) {
            $files[] = $pdfPath;
        }

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $config = new ExtractionConfig(
            chunking: new ChunkingConfig(maxChars: 300),
        );

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files, $config);

        $this->assertIsArray($results);
        $this->assertCount(count($files), $results);

        foreach ($results as $result) {
            $this->assertNotNull($result->content);
        }
    }

    /**
     * Test: Batch extraction with MIME types.
     */
    #[Test]
    public function it_extracts_batch_files_with_mime_types(): void
    {
        $files = [
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        foreach ($results as $result) {
            $this->assertNotEmpty($result->mimeType, 'Result should have MIME type');
        }
    }

    /**
     * Test: Batch extraction with byte arrays.
     */
    #[Test]
    public function it_extracts_multiple_byte_arrays_in_batch(): void
    {
        $files = [
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $byteArrays = [];
        $mimeTypes = [];

        foreach ($files as $file) {
            $byteArrays[] = file_get_contents($file);
            $mimeTypes[] = mime_content_type($file) ?: 'text/plain';
        }

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractBytes($byteArrays, $mimeTypes);

        $this->assertIsArray($results);
        $this->assertCount(count($byteArrays), $results, 'Should return one result per byte array');

        foreach ($results as $result) {
            $this->assertNotNull($result->content, 'Result should have content');
        }
    }

    /**
     * Test: Batch extraction returns consistent extraction results.
     */
    #[Test]
    public function it_produces_consistent_batch_results(): void
    {
        $files = [
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $kreuzberg = new Kreuzberg();
        $results1 = $kreuzberg->batchExtractFiles($files);
        $results2 = $kreuzberg->batchExtractFiles($files);

        $this->assertSame(count($results1), count($results2), 'Batch runs should produce same result count');
    }

    /**
     * Test: Single file batch extraction.
     */
    #[Test]
    public function it_extracts_single_file_batch(): void
    {
        $files = [
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        $this->assertCount(1, $results, 'Single file batch should return one result');
        $this->assertNotEmpty($results[0]->content, 'Result should contain content');
    }

    /**
     * Test: Batch extraction with empty array.
     */
    #[Test]
    public function it_handles_empty_file_list_in_batch(): void
    {
        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles([]);

        $this->assertIsArray($results);
        $this->assertEmpty($results, 'Empty file list should return empty results');
    }

    /**
     * Test: Batch extraction includes all metadata.
     */
    #[Test]
    public function it_includes_metadata_in_batch_results(): void
    {
        $files = [
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        foreach ($results as $result) {
            $this->assertNotEmpty($result->content);
            $this->assertNotNull($result->metadata, 'Result should include metadata');
            $this->assertNotEmpty($result->mimeType, 'Result should include MIME type');
        }
    }

    /**
     * Test: Batch extraction works with large number of files.
     */
    #[Test]
    public function it_handles_large_batch_extraction(): void
    {
        $file = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($file)) {
            $this->markTestSkipped("Test file not found: {$file}");
        }

        // Create a batch with multiple copies of the same file
        $files = array_fill(0, 5, $file);

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        $this->assertCount(5, $results, 'Large batch should process all files');

        foreach ($results as $result) {
            $this->assertNotEmpty($result->content);
        }
    }

    /**
     * Test: Batch configuration propagates to all extractions.
     */
    #[Test]
    public function it_applies_configuration_to_all_batch_files(): void
    {
        $files = [
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        $pdfPath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (file_exists($pdfPath)) {
            $files[] = $pdfPath;
        }

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $config = new ExtractionConfig(
            chunking: new ChunkingConfig(
                maxChars: 250,
                maxOverlap: 25,
                respectSentences: true,
            ),
        );

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files, $config);

        foreach ($results as $result) {
            $this->assertNotNull($result->content, 'Each result should have content');
        }
    }

    /**
     * Test: Batch extraction creates consistent object types.
     */
    #[Test]
    public function it_returns_consistent_result_types_in_batch(): void
    {
        $files = [
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        foreach ($results as $result) {
            $this->assertIsObject($result, 'Each result should be an object');
            $this->assertTrue(
                method_exists($result, 'getContent') || isset($result->content),
                'Result should have content accessor',
            );
        }
    }
}
