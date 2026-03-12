<?php

declare(strict_types=1);

namespace Kreuzberg\Types;

/**
 * Chunk metadata describing offsets within the original document.
 *
 * @property-read int $byteStart Starting byte offset
 * @property-read int $byteEnd Ending byte offset
 * @property-read int|null $tokenCount Number of tokens in chunk
 * @property-read int $chunkIndex Chunk index (0-based)
 * @property-read int $totalChunks Total number of chunks
 * @property-read int|null $firstPage First page number in chunk
 * @property-read int|null $lastPage Last page number in chunk
 * @property-read HeadingContext|null $headingContext Heading hierarchy for this chunk's section
 */
readonly class ChunkMetadata
{
    public function __construct(
        public int $byteStart,
        public int $byteEnd,
        public ?int $tokenCount,
        public int $chunkIndex,
        public int $totalChunks,
        public ?int $firstPage = null,
        public ?int $lastPage = null,
        public ?HeadingContext $headingContext = null,
    ) {
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        /** @var int $byteStart */
        $byteStart = $data['byte_start'] ?? 0;

        /** @var int $byteEnd */
        $byteEnd = $data['byte_end'] ?? 0;

        /** @var int|null $tokenCount */
        $tokenCount = $data['token_count'] ?? null;

        /** @var int $chunkIndex */
        $chunkIndex = $data['chunk_index'] ?? 0;

        /** @var int $totalChunks */
        $totalChunks = $data['total_chunks'] ?? 0;

        /** @var int|null $firstPage */
        $firstPage = $data['first_page'] ?? null;

        /** @var int|null $lastPage */
        $lastPage = $data['last_page'] ?? null;

        /** @var array<string, mixed>|null $rawHeadingContext */
        $rawHeadingContext = is_array($data['heading_context'] ?? null) ? $data['heading_context'] : null;
        $headingContext = $rawHeadingContext !== null
            ? HeadingContext::fromArray($rawHeadingContext)
            : null;

        return new self(
            byteStart: $byteStart,
            byteEnd: $byteEnd,
            tokenCount: $tokenCount,
            chunkIndex: $chunkIndex,
            totalChunks: $totalChunks,
            firstPage: $firstPage,
            lastPage: $lastPage,
            headingContext: $headingContext,
        );
    }
}
