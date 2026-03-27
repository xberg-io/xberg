<?php

declare(strict_types=1);

namespace Kreuzberg\Tests\Unit\Config;

use Kreuzberg\Config\ChunkingConfig;
use PHPUnit\Framework\Attributes\CoversClass;
use PHPUnit\Framework\Attributes\Group;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * Unit tests for ChunkingConfig class.
 *
 * Tests construction with default values and property access.
 * The ChunkingConfig is defined by the extension and only supports:
 * - Constructor with no parameters
 * - Properties: maxChars (default 512), maxOverlap (default 50)
 *
 * Test Coverage:
 * - Construction with default values
 * - Property access (maxChars, maxOverlap, respectSentences, respectParagraphs)
 */
#[CoversClass(ChunkingConfig::class)]
#[Group('unit')]
#[Group('config')]
final class ChunkingConfigTest extends TestCase
{
    #[Test]
    public function it_creates_with_default_values(): void
    {
        $config = new ChunkingConfig();

        $this->assertSame(512, $config->maxChars);
        $this->assertSame(50, $config->maxOverlap);
        $this->assertTrue($config->respectSentences);
        $this->assertTrue($config->respectParagraphs);
    }

    #[Test]
    public function it_can_access_max_chars_property(): void
    {
        $config = new ChunkingConfig();

        $this->assertIsInt($config->maxChars);
        $this->assertSame(512, $config->maxChars);
    }

    #[Test]
    public function it_can_access_max_overlap_property(): void
    {
        $config = new ChunkingConfig();

        $this->assertIsInt($config->maxOverlap);
        $this->assertSame(50, $config->maxOverlap);
    }

    #[Test]
    public function it_can_access_respect_sentences_property(): void
    {
        $config = new ChunkingConfig();

        $this->assertIsBool($config->respectSentences);
        $this->assertTrue($config->respectSentences);
    }

    #[Test]
    public function it_can_access_respect_paragraphs_property(): void
    {
        $config = new ChunkingConfig();

        $this->assertIsBool($config->respectParagraphs);
        $this->assertTrue($config->respectParagraphs);
    }

    #[Test]
    public function it_defaults_chunker_type_to_text(): void
    {
        $config = new ChunkingConfig();

        $this->assertSame('text', $config->chunkerType);
    }

    #[Test]
    public function it_accepts_markdown_chunker_type(): void
    {
        $config = new ChunkingConfig(chunkerType: 'markdown');

        $this->assertSame('markdown', $config->chunkerType);
    }

    #[Test]
    public function it_round_trips_chunker_type_through_array(): void
    {
        $config = ChunkingConfig::fromArray(['chunker_type' => 'markdown']);

        $this->assertSame('markdown', $config->chunkerType);
        $this->assertArrayHasKey('chunker_type', $config->toArray());
        $this->assertSame('markdown', $config->toArray()['chunker_type']);
    }

    #[Test]
    public function it_omits_default_chunker_type_from_array(): void
    {
        $config = new ChunkingConfig();

        $this->assertArrayNotHasKey('chunker_type', $config->toArray());
    }
}
