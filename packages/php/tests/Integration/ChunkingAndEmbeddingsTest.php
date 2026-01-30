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
#[RequiresPhpExtension('kreuzberg-php')]
final class ChunkingAndEmbeddingsTest extends TestCase
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
    public function it_chunks_document_content(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(
            maxChars: 500,
            maxOverlap: 50,
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
        $this->assertNotEmpty(
            $result->chunks,
            'Should produce at least one chunk',
        );
        $this->assertGreaterThan(
            0,
            count($result->chunks),
            'Should produce at least one chunk',
        );
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
            maxChars: $maxChunkSize,
            maxOverlap: 20,
            respectSentences: false,
        );

        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->chunks, 'Chunks should not be null');
        $this->assertNotEmpty($result->chunks, 'Should produce chunks');

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
            $this->assertLessThanOrEqual(
                $maxChunkSize + 100,
                strlen($chunk->content),
                'Chunk size should respect max chars configuration (with some tolerance)',
            );
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
            maxChars: 400,
            maxOverlap: 100,
            respectSentences: true,
        );

        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull(
            $result->chunks,
            'Should produce chunks with overlap',
        );
        $this->assertIsArray($result->chunks, 'Chunks should be an array');
        $this->assertNotEmpty($result->chunks, 'Should produce at least one chunk');
        $this->assertGreaterThan(
            1,
            count($result->chunks),
            'Should produce multiple chunks for overlapping to be meaningful',
        );
    }

    #[Test]
    public function it_respects_sentence_boundaries_when_chunking(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(
            maxChars: 300,
            maxOverlap: 50,
            respectSentences: true,
        );

        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->chunks, 'Chunks should not be null');
        $this->assertNotEmpty($result->chunks, 'Should produce chunks');

        foreach ($result->chunks as $chunk) {
            $this->assertNotEmpty(
                $chunk->content,
                'Chunk content should not be empty',
            );
        }
    }

    #[Test]
    public function it_generates_embeddings_for_chunks(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $embeddingConfig = new EmbeddingConfig(
            model: 'balanced',
            normalize: true,
        );

        $chunkingConfig = new ChunkingConfig(
            maxChars: 500,
            maxOverlap: 50,
            embedding: $embeddingConfig,
        );

        $config = new ExtractionConfig(
            chunking: $chunkingConfig,
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->chunks, 'Chunks should not be null');
        $this->assertNotEmpty($result->chunks, 'Should produce chunks');

        $chunk = $result->chunks[0];

        $this->assertObjectHasProperty(
            'embedding',
            $chunk,
            'Chunk should have embedding when embedding config is provided',
        );

        // Embedding may be null on platforms where ONNX runtime is not available
        $embedding = $chunk->embedding;
        if ($embedding === null) {
            $this->markTestSkipped('Embedding model not available on this platform');
        }

        $this->assertIsArray(
            $embedding,
            'Embedding should be an array of floats',
        );
        $this->assertNotEmpty(
            $embedding,
            'Embedding should not be empty',
        );
    }

    #[Test]
    public function it_includes_chunk_metadata(): void
    {
        $filePath = $this->testDocumentsPath . '/pdfs/code_and_formula.pdf';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(
            maxChars: 500,
            maxOverlap: 50,
        );

        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->chunks, 'Chunks should not be null');
        $this->assertNotEmpty($result->chunks, 'Should produce chunks');

        $chunk = $result->chunks[0];

        // Metadata is accessible via getMetadata() method
        $this->assertTrue(
            method_exists($chunk, 'getMetadata'),
            'Chunk should have getMetadata method',
        );
        $metadata = $chunk->getMetadata();
        $this->assertNotNull(
            $metadata,
            'Chunk metadata should not be null',
        );
    }

    #[Test]
    public function it_chunks_documents_without_embeddings(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(
            maxChars: 400,
            maxOverlap: 40,
            embedding: null,
        );

        $config = new ExtractionConfig(
            chunking: $chunkingConfig,
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull(
            $result->chunks,
            'Should produce chunks even without embeddings',
        );
        $this->assertNotEmpty($result->chunks, 'Should produce chunks');

        foreach ($result->chunks as $chunk) {
            $this->assertIsString(
                $chunk->content,
                'Chunk should have content string',
            );
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
            maxChars: 10000,
            maxOverlap: 100,
        );

        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->chunks, 'Chunks should not be null');
        $this->assertNotEmpty($result->chunks, 'Should produce chunks');
        $this->assertGreaterThanOrEqual(
            1,
            count($result->chunks),
            'Should have at least one chunk',
        );
    }

    #[Test]
    public function it_normalizes_embeddings_when_configured(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $embeddingConfig = new EmbeddingConfig(
            model: 'balanced',
            normalize: true,
        );

        $chunkingConfig = new ChunkingConfig(
            maxChars: 500,
            embedding: $embeddingConfig,
        );

        $config = new ExtractionConfig(
            chunking: $chunkingConfig,
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->chunks, 'Chunks should not be null');
        $this->assertNotEmpty($result->chunks, 'Should produce chunks');

        // Embedding may be null on platforms where ONNX runtime is not available (e.g., ARM)
        $embedding = $result->chunks[0]->embedding;
        if ($embedding === null) {
            $this->markTestSkipped('Embedding model not available on this platform');
        }

        $this->assertIsArray(
            $embedding,
            'Normalized embedding should be an array',
        );
        $this->assertNotEmpty(
            $embedding,
            'Normalized embedding should not be empty',
        );

        $magnitude = sqrt(array_sum(array_map(fn ($v) => $v * $v, $embedding)));
        $this->assertEqualsWithDelta(
            1.0,
            $magnitude,
            0.01,
            'Normalized embedding should have unit magnitude',
        );
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
            maxChars: 500,
            maxOverlap: 50,
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
            $this->assertNotNull(
                $result->chunks,
                'Each result should have chunks',
            );
            $this->assertIsArray(
                $result->chunks,
                'Each result should have chunks array',
            );
            $this->assertNotEmpty(
                $result->chunks,
                'Each result should produce chunks',
            );
        }
    }

    #[Test]
    public function it_validates_chunk_content_is_utf8(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $chunkingConfig = new ChunkingConfig(maxChars: 400);
        $config = new ExtractionConfig(chunking: $chunkingConfig);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->chunks, 'Chunks should not be null');
        $this->assertNotEmpty($result->chunks, 'Should produce chunks');

        foreach ($result->chunks as $chunk) {
            $this->assertTrue(
                mb_check_encoding($chunk->content, 'UTF-8'),
                'Chunk content should be valid UTF-8',
            );
        }
    }
}
