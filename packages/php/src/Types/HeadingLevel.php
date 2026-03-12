<?php

declare(strict_types=1);

namespace Kreuzberg\Types;

/**
 * A single heading in the document hierarchy.
 *
 * @property-read int $level Heading depth (1 = h1, 2 = h2, etc.)
 * @property-read string $text Text content of the heading
 */
readonly class HeadingLevel
{
    public function __construct(
        public int $level,
        public string $text,
    ) {
    }

    /**
     * @param array<string, mixed> $data
     */
    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        /** @var int $level */
        $level = isset($data['level']) && is_int($data['level']) ? $data['level'] : 0;
        /** @var string $text */
        $text = isset($data['text']) && is_string($data['text']) ? $data['text'] : '';

        return new self(
            level: $level,
            text: $text,
        );
    }
}
