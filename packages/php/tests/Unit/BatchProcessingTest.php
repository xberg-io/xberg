<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Unit;

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Kreuzberg;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Behavior-driven tests for batch processing functionality.
 *
 * Tests the parallel extraction of multiple documents.
 */
#[CoversClass(Kreuzberg::class)]
#[Group('unit')]
#[Group('batch')]
#[RequiresPhpExtension('kreuzberg-php')]
final class BatchProcessingTest extends TestCase
{
    private string $testDocumentsPath;

    protected function setUp(): void
    {
        if (!extension_loaded('kreuzberg-php')) {
            $this->markTestSkipped('Kreuzberg extension is not loaded');
        }

        $this->testDocumentsPath = dirname(__DIR__, 4) . DIRECTORY_SEPARATOR . 'test_documents';
    }

    #[Test]
    public function it_extracts_multiple_files_in_batch(): void
    {
        $files = [
            $this->testDocumentsPath . '/pdfs/code_and_formula.pdf',
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        $this->assertIsArray($results, 'Batch extraction should return an array');
        $this->assertCount(2, $results, 'Should return one result per input file');

        foreach ($results as $index => $result) {
            $this->assertNotEmpty(
                $result->content,
                "Result {$index} should have content",
            );
            $this->assertNotEmpty(
                $result->mimeType,
                "Result {$index} should have MIME type",
            );
        }
    }

    #[Test]
    public function it_extracts_multiple_byte_arrays_in_batch(): void
    {
        $files = [
            $this->testDocumentsPath . '/pdfs/code_and_formula.pdf',
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $dataList = array_map(fn ($file) => file_get_contents($file), $files);
        $mimeTypes = ['application/pdf', 'text/markdown'];

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractBytes($dataList, $mimeTypes);

        $this->assertIsArray($results);
        $this->assertCount(2, $results, 'Should return one result per input byte array');

        foreach ($results as $index => $result) {
            $this->assertNotEmpty(
                $result->content,
                "Result {$index} should have extracted content",
            );
        }
    }

    #[Test]
    public function it_returns_results_in_same_order_as_input(): void
    {
        $files = [
            $this->testDocumentsPath . '/pdfs/code_and_formula.pdf',
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        $this->assertStringContainsString(
            'pdf',
            strtolower($results[0]->mimeType),
            'First result should correspond to first file (PDF)',
        );

        $this->assertStringContainsString(
            'text',
            strtolower($results[1]->mimeType),
            'Second result should correspond to second file (Markdown)',
        );
    }

    #[Test]
    public function it_applies_config_to_all_batch_files(): void
    {
        $files = [
            $this->testDocumentsPath . '/pdfs/code_and_formula.pdf',
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $config = new ExtractionConfig(forceOcr: true);
        $kreuzberg = new Kreuzberg($config);
        $results = $kreuzberg->batchExtractFiles($files);

        foreach ($results as $index => $result) {
            $this->assertNotNull(
                $result->content,
                "Result {$index} should have content when config is applied",
            );
        }
    }

    #[Test]
    public function it_handles_empty_batch_gracefully(): void
    {
        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles([]);

        $this->assertIsArray($results, 'Empty batch should return empty array');
        $this->assertEmpty($results, 'Result should be empty for empty input');
    }

    #[Test]
    public function it_handles_single_file_batch(): void
    {
        $files = [$this->testDocumentsPath . '/pdfs/code_and_formula.pdf'];

        if (!file_exists($files[0])) {
            $this->markTestSkipped("Test file not found: {$files[0]}");
        }

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        $this->assertCount(1, $results, 'Single file batch should return one result');
        $this->assertNotEmpty($results[0]->content);
    }

    /**
     * Batch operations return error results in metadata for individual failures
     * rather than throwing exceptions. This allows the batch to continue.
     */
    #[Test]
    public function it_handles_batch_with_nonexistent_file_gracefully(): void
    {
        $validFile = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (!file_exists($validFile)) {
            $this->markTestSkipped("Test file not found: {$validFile}");
        }

        $files = [
            $validFile,
            '/nonexistent/file.pdf',
        ];

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        $this->assertCount(2, $results, 'Should return results for all files');
        $this->assertNotEmpty($results[0]->content, 'Valid file should have content');
        $hasError = str_starts_with($results[1]->content, 'Error:');
        $this->assertTrue($hasError, 'Invalid file result should indicate error');
    }

    #[Test]
    public function it_requires_matching_mime_types_count_for_batch_bytes(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';
        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $dataList = [
            file_get_contents($filePath),
            file_get_contents($filePath),
        ];
        $mimeTypes = ['application/pdf'];

        $this->expectException(KreuzbergException::class);

        $kreuzberg = new Kreuzberg();
        $kreuzberg->batchExtractBytes($dataList, $mimeTypes);
    }

    #[Test]
    public function it_processes_multiple_pdfs_in_batch(): void
    {
        $files = [
            $this->testDocumentsPath . '/pdfs/code_and_formula.pdf',
        ];

        $secondPdf = $this->testDocumentsPath . '/pdfs_with_tables/simple.pdf';
        if (file_exists($secondPdf)) {
            $files[] = $secondPdf;
        }

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        $this->assertCount(count($files), $results);

        foreach ($results as $result) {
            $this->assertStringContainsString(
                'pdf',
                strtolower($result->mimeType),
                'All results should be PDFs',
            );
            $this->assertNotEmpty($result->content);
        }
    }

    #[Test]
    public function it_handles_batch_with_different_document_types(): void
    {
        $files = [
            $this->testDocumentsPath . '/pdfs/code_and_formula.pdf',
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        if (file_exists($this->testDocumentsPath . '/extraction_test.odt')) {
            $files[] = $this->testDocumentsPath . '/extraction_test.odt';
        }

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $kreuzberg = new Kreuzberg();
        $results = $kreuzberg->batchExtractFiles($files);

        $this->assertCount(
            count($files),
            $results,
            'Should extract all different document types',
        );

        foreach ($results as $result) {
            $this->assertNotEmpty(
                $result->content,
                'Each document type should have extracted content',
            );
        }
    }

    #[Test]
    public function it_batch_extracts_with_method_override_config(): void
    {
        $files = [
            $this->testDocumentsPath . '/pdfs/code_and_formula.pdf',
            $this->testDocumentsPath . '/extraction_test.md',
        ];

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $defaultConfig = new ExtractionConfig(forceOcr: false);
        $overrideConfig = new ExtractionConfig(forceOcr: true);

        $kreuzberg = new Kreuzberg($defaultConfig);
        $results = $kreuzberg->batchExtractFiles($files, $overrideConfig);

        foreach ($results as $result) {
            $this->assertNotNull(
                $result->content,
                'Override config should be applied to all batch items',
            );
        }
    }
}
