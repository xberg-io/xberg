<?php

declare(strict_types=1);

namespace Kreuzberg\Types;

/**
 * Heading context for a chunk's section in the document.
 *
 * @property-read list<HeadingLevel> $headings Heading hierarchy from document root to this chunk's section
 */
readonly class HeadingContext
{
    /**
     * @param list<HeadingLevel> $headings
     */
    public function __construct(
        public array $headings,
    ) {
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        /** @var list<array<string, mixed>> $rawHeadings */
        $rawHeadings = is_array($data['headings'] ?? null) ? $data['headings'] : [];
        /** @var list<HeadingLevel> $headings */
        $headings = array_values(array_map(
            static fn (array $h): HeadingLevel => HeadingLevel::fromArray($h),
            $rawHeadings,
        ));

        return new self(headings: $headings);
    }
}
