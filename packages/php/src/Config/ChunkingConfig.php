<?php

declare(strict_types=1);

namespace Kreuzberg\Config;

/**
 * Text chunking configuration.
 */
readonly class ChunkingConfig
{
    public function __construct(
        /**
         * Maximum number of characters per text chunk.
         *
         * Defines the maximum number of characters per chunk. Larger chunks retain
         * more context but may be too long for some downstream processing.
         *
         * Valid range: 1-unlimited (practical range: 128-4096)
         * Recommended values:
         * - 256-512: For short documents or limited context windows
         * - 512-1024: For standard documents and embeddings
         * - 1024-2048: For longer documents requiring more context
         *
         * @var int
         * @default 512
         */
        public int $maxChars = 512,

        /**
         * Number of characters to overlap between consecutive chunks.
         *
         * Overlap ensures context continuity between chunks by repeating some text
         * at the boundaries. This helps maintain semantic coherence when chunks
         * are processed independently.
         *
         * Valid range: 0-$maxChars
         * Recommended values:
         * - 0-10%: For minimal overlap
         * - 10-25%: For standard overlap (typically 50 chars for 512-char chunks)
         * - 25-50%: For high overlap when context preservation is critical
         *
         * @var int
         * @default 50
         */
        public int $maxOverlap = 50,

        /**
         * Respect sentence boundaries when creating chunks.
         *
         * When enabled, chunks will not split in the middle of sentences.
         * This ensures that no sentence is broken across multiple chunks,
         * maintaining semantic integrity. Actual chunk size may vary slightly
         * to preserve complete sentences.
         *
         * @var bool
         * @default true
         */
        public bool $respectSentences = true,

        /**
         * Respect paragraph boundaries when creating chunks.
         *
         * When enabled, chunks will avoid splitting paragraphs.
         * This preserves thematic grouping and context within the document.
         * May result in variable chunk sizes to maintain paragraph integrity.
         *
         * @var bool
         * @default true
         */
        public bool $respectParagraphs = true,

        /**
         * Embedding configuration for chunks.
         *
         * When provided, embeddings will be generated for each chunk using the
         * specified embedding model. This allows for semantic search over chunks.
         *
         * @var EmbeddingConfig|null
         * @default null
         */
        public ?EmbeddingConfig $embedding = null,

        /**
         * Sizing type: "characters" (default) or "tokenizer".
         *
         * When set to "tokenizer", chunks are sized by token count using the
         * specified HuggingFace tokenizer model.
         *
         * @var string
         * @default "characters"
         */
        public string $sizingType = 'characters',

        /**
         * HuggingFace model ID for tokenizer sizing.
         *
         * Only used when sizingType is "tokenizer".
         * Example: "Xenova/gpt-4o", "bert-base-uncased"
         *
         * @var string|null
         * @default null
         */
        public ?string $sizingModel = null,

        /**
         * Optional cache directory for tokenizer files.
         *
         * @var string|null
         * @default null
         */
        public ?string $sizingCacheDir = null,

        /**
         * Type of chunker to use.
         *
         * Supported values: "text" (default), "markdown", "yaml", "semantic".
         * Set to "semantic" for topic-aware chunking that works out of the box
         * with sensible defaults (maxChars=1000, maxOverlap=200,
         * topicThreshold=0.75). No other parameters needed.
         *
         * @var string
         * @default "text"
         */
        public string $chunkerType = 'text',

        /**
         * Cosine similarity threshold for semantic topic detection.
         * Optional, defaults to 0.75. Only used with chunkerType "semantic"
         * and embeddings. Rarely needs tuning. Range: 0.0-1.0.
         *
         * @var float|null
         * @default null (uses 0.75)
         */
        public ?float $topicThreshold = null,

        /**
         * Prepend heading context to each chunk for improved retrieval.
         *
         * When enabled, each chunk will be prefixed with the heading hierarchy
         * context from the source document. This improves retrieval quality
         * by making each chunk self-contained.
         *
         * @var bool
         * @default false
         */
        public bool $prependHeadingContext = false,
    ) {
    }

    /**
     * Create configuration from array data.
     *
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        /** @var int $maxChars */
        $maxChars = $data['max_chars'] ?? 512;
        if (!is_int($maxChars)) {
            /** @var int $maxChars */
            $maxChars = (int) $maxChars;
        }

        /** @var int $maxOverlap */
        $maxOverlap = $data['max_overlap'] ?? 50;
        if (!is_int($maxOverlap)) {
            /** @var int $maxOverlap */
            $maxOverlap = (int) $maxOverlap;
        }

        /** @var bool $respectSentences */
        $respectSentences = $data['respect_sentences'] ?? true;
        if (!is_bool($respectSentences)) {
            /** @var bool $respectSentences */
            $respectSentences = (bool) $respectSentences;
        }

        /** @var bool $respectParagraphs */
        $respectParagraphs = $data['respect_paragraphs'] ?? true;
        if (!is_bool($respectParagraphs)) {
            /** @var bool $respectParagraphs */
            $respectParagraphs = (bool) $respectParagraphs;
        }

        $embedding = null;
        if (isset($data['embedding']) && is_array($data['embedding'])) {
            /** @var array<string, mixed> $embeddingData */
            $embeddingData = $data['embedding'];
            $embedding = EmbeddingConfig::fromArray($embeddingData);
        }

        /** @var string $sizingType */
        $sizingType = 'characters';
        /** @var string|null $sizingModel */
        $sizingModel = null;
        /** @var string|null $sizingCacheDir */
        $sizingCacheDir = null;
        if (isset($data['sizing']) && is_array($data['sizing'])) {
            /** @var array<string, mixed> $sizingData */
            $sizingData = $data['sizing'];
            if (isset($sizingData['type']) && is_string($sizingData['type'])) {
                $sizingType = $sizingData['type'];
            }
            if (isset($sizingData['model']) && is_string($sizingData['model'])) {
                $sizingModel = $sizingData['model'];
            }
            if (isset($sizingData['cache_dir']) && is_string($sizingData['cache_dir'])) {
                $sizingCacheDir = $sizingData['cache_dir'];
            }
        }

        /** @var string $chunkerType */
        $chunkerType = $data['chunker_type'] ?? 'text';
        if (!is_string($chunkerType)) {
            /** @var string $chunkerType */
            $chunkerType = (string) $chunkerType;
        }

        /** @var float|null $topicThreshold */
        $topicThreshold = isset($data['topic_threshold']) && is_numeric($data['topic_threshold'])
            ? (float) $data['topic_threshold']
            : null;

        /** @var bool $prependHeadingContext */
        $prependHeadingContext = $data['prepend_heading_context'] ?? false;
        if (!is_bool($prependHeadingContext)) {
            /** @var bool $prependHeadingContext */
            $prependHeadingContext = (bool) $prependHeadingContext;
        }

        return new self(
            maxChars: $maxChars,
            maxOverlap: $maxOverlap,
            respectSentences: $respectSentences,
            respectParagraphs: $respectParagraphs,
            embedding: $embedding,
            sizingType: $sizingType,
            sizingModel: $sizingModel,
            sizingCacheDir: $sizingCacheDir,
            chunkerType: $chunkerType,
            topicThreshold: $topicThreshold,
            prependHeadingContext: $prependHeadingContext,
        );
    }

    /**
     * Create configuration from JSON string.
     */
    public static function fromJson(string $json): self
    {
        $data = json_decode($json, true);
        if (json_last_error() !== JSON_ERROR_NONE) {
            throw new \InvalidArgumentException('Invalid JSON: ' . json_last_error_msg());
        }
        if (!is_array($data)) {
            throw new \InvalidArgumentException('JSON must decode to an object/array');
        }
        /** @var array<string, mixed> $data */
        return self::fromArray($data);
    }

    /**
     * Create configuration from JSON file.
     */
    public static function fromFile(string $path): self
    {
        if (!file_exists($path)) {
            throw new \InvalidArgumentException("File not found: {$path}");
        }
        $contents = file_get_contents($path);
        if ($contents === false) {
            throw new \InvalidArgumentException("Unable to read file: {$path}");
        }
        return self::fromJson($contents);
    }

    /**
     * @return array<string, mixed>
     */
    public function toArray(): array
    {
        // Use toRustArray() for embedding to get Rust-compatible format
        $embedding = $this->embedding !== null ? $this->embedding->toRustArray() : null;

        $sizing = null;
        if ($this->sizingType !== 'characters') {
            $sizing = array_filter([
                'type' => $this->sizingType,
                'model' => $this->sizingModel,
                'cache_dir' => $this->sizingCacheDir,
            ], static fn ($value): bool => $value !== null);
        }

        $result = array_filter([
            'max_chars' => $this->maxChars,
            'max_overlap' => $this->maxOverlap,
            'respect_sentences' => $this->respectSentences,
            'respect_paragraphs' => $this->respectParagraphs,
            'embedding' => $embedding,
            'chunker_type' => $this->chunkerType !== 'text' ? $this->chunkerType : null,
            'topic_threshold' => $this->topicThreshold,
            'sizing' => $sizing,
            'prepend_heading_context' => $this->prependHeadingContext,
        ], static fn ($value): bool => $value !== null);

        return $result;
    }

    /**
     * Convert configuration to JSON string.
     */
    public function toJson(): string
    {
        $json = json_encode($this->toArray(), JSON_PRETTY_PRINT);
        if ($json === false) {
            throw new \RuntimeException('Failed to encode configuration to JSON');
        }
        return $json;
    }
}
