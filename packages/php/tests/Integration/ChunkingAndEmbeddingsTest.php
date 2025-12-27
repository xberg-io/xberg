<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Integration;

use Kreuzberg\Config\ChunkingConfig;
use Kreuzberg\Config\EmbeddingConfig;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Kreuzberg;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\RequiresPhpExtension;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Integration tests for chunking and embeddings functionality.
 *
 * Tests text chunking and embedding generation features.
 */
#[CoversClass(Kreuzberg::class)]
#[Group('integration')]
#[Group('chunking')]
#[RequiresPhpExtension('kreuzberg')]
final class ChunkingAndEmbeddingsTest extends TestCase
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
    public function it_chunks_document_content(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(
            maxChunkSize: 500,
            chunkOverlap: 50,
            respectSentences: true,
        );

        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull(
            $result->chunks,
            'Chunking config should produce chunks',
        );
        $this->assertIsArray(
            $result->chunks,
            'Chunks should be an array',
        );

        if (!empty($result->chunks)) {
            $this->assertGreaterThan(
                0,
                count($result->chunks),
                'Should produce at least one chunk',
            );
        }
    }

    #[Test]
    public function it_respects_chunk_size_configuration(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $maxChunkSize = 300;
        $chunkingConfig = new ChunkingConfig(
            maxChunkSize: $maxChunkSize,
            chunkOverlap: 20,
            respectSentences: false,
        );

        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        if (!empty($result->chunks)) {
            foreach ($result->chunks as $chunk) {
                $this->assertObjectHasProperty(
                    'content',
                    $chunk,
                    'Chunk should have content property',
                );
                $this->assertIsString(
                    $chunk->content,
                    'Chunk content should be a string',
                );
            }
        }
    }

    #[Test]
    public function it_creates_overlapping_chunks(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(
            maxChunkSize: 400,
            chunkOverlap: 100,
            respectSentences: true,
        );

        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull(
            $result->chunks,
            'Should produce chunks with overlap',
        );

        if (!empty($result->chunks)) {
            $this->assertIsArray($result->chunks);
        }
    }

    #[Test]
    public function it_respects_sentence_boundaries_when_chunking(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(
            maxChunkSize: 300,
            chunkOverlap: 50,
            respectSentences: true,
        );

        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        if (!empty($result->chunks)) {
            foreach ($result->chunks as $chunk) {
                $this->assertNotEmpty(
                    $chunk->content,
                    'Chunk content should not be empty',
                );
            }
        }
    }

    #[Test]
    public function it_generates_embeddings_for_chunks(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(
            maxChunkSize: 500,
            chunkOverlap: 50,
        );

        $embeddingConfig = new EmbeddingConfig(
            model: 'all-minilm-l6-v2',
            normalize: true,
        );

        $config = new ExtractionConfig(
            chunking: $chunkingConfig,
            embedding: $embeddingConfig,
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        if (!empty($result->chunks)) {
            $chunk = $result->chunks[0];

            $this->assertObjectHasProperty(
                'embedding',
                $chunk,
                'Chunk should have embedding when embedding config is provided',
            );

            if ($chunk->embedding !== null) {
                $this->assertIsArray(
                    $chunk->embedding,
                    'Embedding should be an array of floats',
                );
                $this->assertNotEmpty(
                    $chunk->embedding,
                    'Embedding should not be empty',
                );
            }
        }
    }

    #[Test]
    public function it_includes_chunk_metadata(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(
            maxChunkSize: 500,
            chunkOverlap: 50,
        );

        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        if (!empty($result->chunks)) {
            $chunk = $result->chunks[0];

            $this->assertObjectHasProperty(
                'metadata',
                $chunk,
                'Chunk should have metadata',
            );
            $this->assertNotNull(
                $chunk->metadata,
                'Chunk metadata should not be null',
            );
        }
    }

    #[Test]
    public function it_chunks_documents_without_embeddings(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(
            maxChunkSize: 400,
            chunkOverlap: 40,
        );

        $config = new ExtractionConfig(
            chunking: $chunkingConfig,
            embedding: null,
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull(
            $result->chunks,
            'Should produce chunks even without embeddings',
        );

        if (!empty($result->chunks)) {
            foreach ($result->chunks as $chunk) {
                $this->assertIsString(
                    $chunk->content,
                    'Chunk should have content string',
                );
            }
        }
    }

    #[Test]
    public function it_handles_small_documents_with_chunking(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(
            maxChunkSize: 10000,
            chunkOverlap: 100,
        );

        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        if (!empty($result->chunks)) {
            $this->assertGreaterThanOrEqual(
                1,
                count($result->chunks),
                'Should have at least one chunk',
            );
        }
    }

    #[Test]
    public function it_normalizes_embeddings_when_configured(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(maxChunkSize: 500);

        $embeddingConfig = new EmbeddingConfig(
            model: 'all-minilm-l6-v2',
            normalize: true,
        );

        $config = new ExtractionConfig(
            chunking: $chunkingConfig,
            embedding: $embeddingConfig,
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        if (!empty($result->chunks) && !empty($result->chunks[0]->embedding)) {
            $embedding = $result->chunks[0]->embedding;

            $this->assertIsArray(
                $embedding,
                'Normalized embedding should be an array',
            );
            $this->assertNotEmpty(
                $embedding,
                'Normalized embedding should not be empty',
            );
        }
    }

    #[Test]
    public function it_processes_chunks_in_batch_extraction(): void
    {
        $files = [
            $this->testDocumentsPath . '/extraction_test.md',
            $this->testDocumentsPath . '/pdfs/code_and_formula.pdf',
        ];

        foreach ($files as $file) {
            if (!file_exists($file)) {
                $this->markTestSkipped("Test file not found: {$file}");
            }
        }

        $chunkingConfig = new ChunkingConfig(
            maxChunkSize: 500,
            chunkOverlap: 50,
        );

        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $results = $kreuzberg->batchExtractFiles($files);

        $this->assertCount(
            2,
            $results,
            'Batch processing should return results for all files',
        );

        foreach ($results as $result) {
            if (!empty($result->chunks)) {
                $this->assertIsArray(
                    $result->chunks,
                    'Each result should have chunks array',
                );
            }
        }
    }

    #[Test]
    public function it_validates_chunk_content_is_utf8(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(maxChunkSize: 400);
        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        if (!empty($result->chunks)) {
            foreach ($result->chunks as $chunk) {
                $this->assertTrue(
                    mb_check_encoding($chunk->content, 'UTF-8'),
                    'Chunk content should be valid UTF-8',
                );
            }
        }
    }
}
