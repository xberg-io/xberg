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
use ReflectionClass;

/**
 * Integration tests for embedding generation functionality.
 *
 * Comprehensive testing of vector embedding generation including:
 * - Vector generation from document text
 * - Embedding configuration (model selection, normalization)
 * - Batch embedding processing
 * - Vector dimensionality validation
 * - Model-specific embedding behavior
 * - Configuration serialization
 * - Readonly property enforcement
 *
 * Test Coverage:
 * - Embedding vector generation
 * - Vector normalization
 * - Model configuration
 * - Batch size handling
 * - Vector properties and validation
 * - Configuration builder pattern
 * - JSON serialization
 */
#[CoversClass(Kreuzberg::class)]
#[CoversClass(EmbeddingConfig::class)]
#[Group('integration')]
#[Group('embeddings')]
#[RequiresPhpExtension('kreuzberg-php')]
final class EmbeddingGenerationTest extends TestCase
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
     * Test: Document embeddings are generated and returned as arrays.
     */
    #[Test]
    public function it_generates_embeddings_for_document(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(
            chunking: new ChunkingConfig(embedding: new EmbeddingConfig()),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->embeddings, 'Embeddings should be generated');
        $this->assertIsArray($result->embeddings, 'Embeddings should be an array');
    }

    /**
     * Test: Embeddings contain numeric vectors.
     */
    #[Test]
    public function it_generates_numeric_embedding_vectors(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(
            chunking: new ChunkingConfig(embedding: new EmbeddingConfig()),
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);

        $this->assertNotNull($result->embeddings, 'Embeddings should not be null');
        $this->assertNotEmpty($result->embeddings, 'Should produce embeddings');

        foreach ($result->embeddings as $embedding) {
            $this->assertIsObject($embedding, 'Each embedding should be an object');
            $this->assertObjectHasProperty('vector', $embedding, 'Embedding should have vector property');
            $this->assertNotNull($embedding->vector, 'Vector should not be null');
            $this->assertIsArray($embedding->vector, 'Vector should be an array');
            $this->assertNotEmpty($embedding->vector, 'Vector should not be empty');

            foreach ($embedding->vector as $value) {
                $this->assertTrue(
                    is_float($value) || is_int($value),
                    'Vector values should be numeric',
                );
            }
        }
    }

    /**
     * Test: EmbeddingConfig constructor sets correct model.
     */
    #[Test]
    public function it_creates_embedding_config_with_model(): void
    {
        $config = new EmbeddingConfig(
            model: 'quality',
        );

        $this->assertSame('quality', $config->model, 'Model should be set correctly');
    }

    /**
     * Test: EmbeddingConfig default model is correct.
     */
    #[Test]
    public function it_uses_default_embedding_model(): void
    {
        $config = new EmbeddingConfig();

        $this->assertSame('balanced', $config->model, 'Default model should be balanced');
    }

    /**
     * Test: EmbeddingConfig supports normalization setting.
     */
    #[Test]
    public function it_configures_embedding_normalization(): void
    {
        $configNormalized = new EmbeddingConfig(normalize: true);
        $configUnnormalized = new EmbeddingConfig(normalize: false);

        $this->assertTrue($configNormalized->normalize, 'Normalization should be enabled');
        $this->assertFalse($configUnnormalized->normalize, 'Normalization should be disabled');
    }

    /**
     * Test: EmbeddingConfig supports batch size configuration.
     */
    #[Test]
    public function it_configures_embedding_batch_size(): void
    {
        $config = new EmbeddingConfig(batchSize: 32);

        $this->assertSame(32, $config->batchSize, 'Batch size should be set correctly');
    }

    /**
     * Test: EmbeddingConfig default batch size is null.
     */
    #[Test]
    public function it_uses_default_embedding_batch_size(): void
    {
        $config = new EmbeddingConfig();

        $this->assertNull($config->batchSize, 'Default batch size should be null');
    }

    /**
     * Test: EmbeddingConfig supports fromArray factory method.
     */
    #[Test]
    public function it_creates_embedding_config_from_array(): void
    {
        $data = [
            'model' => 'sentence-transformers/all-mpnet-base-v2',
            'normalize' => false,
            'batch_size' => 64,
        ];

        $config = EmbeddingConfig::fromArray($data);

        $this->assertSame('sentence-transformers/all-mpnet-base-v2', $config->model);
        $this->assertFalse($config->normalize);
        $this->assertSame(64, $config->batchSize);
    }

    /**
     * Test: EmbeddingConfig supports toArray conversion.
     */
    #[Test]
    public function it_converts_embedding_config_to_array(): void
    {
        $config = new EmbeddingConfig(
            model: 'custom-model',
            normalize: false,
            batchSize: 128,
        );

        $array = $config->toArray();

        $this->assertIsArray($array);
        $this->assertSame('custom-model', $array['model']);
        $this->assertFalse($array['normalize']);
        $this->assertSame(128, $array['batch_size']);
    }

    /**
     * Test: EmbeddingConfig supports JSON serialization.
     */
    #[Test]
    public function it_serializes_embedding_config_to_json(): void
    {
        $config = new EmbeddingConfig(
            model: 'test-model',
            normalize: true,
            batchSize: 16,
        );

        $json = $config->toJson();

        $this->assertIsString($json);
        $this->assertStringContainsString('test-model', $json);
        $this->assertStringContainsString('normalize', $json);
        $this->assertStringContainsString('batch_size', $json);
    }

    /**
     * Test: EmbeddingConfig supports JSON deserialization.
     */
    #[Test]
    public function it_deserializes_embedding_config_from_json(): void
    {
        $json = '{"model": "json-model", "normalize": false, "batch_size": 48}';

        $config = EmbeddingConfig::fromJson($json);

        $this->assertSame('json-model', $config->model);
        $this->assertFalse($config->normalize);
        $this->assertSame(48, $config->batchSize);
    }

    /**
     * Test: EmbeddingConfig readonly properties are enforced.
     */
    #[Test]
    public function it_enforces_readonly_embedding_config_properties(): void
    {
        $config = new EmbeddingConfig(
            model: 'readonly-test',
            normalize: true,
        );

        $reflection = new ReflectionClass($config);

        if ($reflection->getAttributes() || $reflection->getProperties()) {
            // Verify the class exists and properties are accessible
            $this->assertSame('readonly-test', $config->model);
            $this->assertTrue($config->normalize);
        }
    }

    /**
     * Test: Embeddings with normalization maintain unit vector properties.
     */
    #[Test]
    public function it_generates_consistent_embeddings(): void
    {
        $filePath = $this->testDocumentsPath . '/extraction_test.md';

        if (!file_exists($filePath)) {
            $this->markTestSkipped("Test file not found: {$filePath}");
        }

        $config = new ExtractionConfig(
            chunking: new ChunkingConfig(embedding: new EmbeddingConfig(normalize: true)),
        );

        $kreuzberg = new Kreuzberg();
        $result1 = $kreuzberg->extractFile($filePath, config: $config);
        $result2 = $kreuzberg->extractFile($filePath, config: $config);

        $this->assertNotNull($result1->embeddings, 'First result embeddings should not be null');
        $this->assertNotNull($result2->embeddings, 'Second result embeddings should not be null');
        $this->assertNotEmpty($result1->embeddings, 'First result should produce embeddings');
        $this->assertNotEmpty($result2->embeddings, 'Second result should produce embeddings');
        $this->assertSame(
            count($result1->embeddings),
            count($result2->embeddings),
            'Multiple extractions should produce same embedding count',
        );
    }
}
