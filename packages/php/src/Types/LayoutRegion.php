<?php

declare(strict_types=1);

namespace Kreuzberg\Types;

/**
 * A detected layout region on a page.
 *
 * When layout detection is enabled, each page may have layout regions
 * identifying different content types (text, pictures, tables, etc.)
 * with confidence scores and spatial positions.
 *
 * @property-read string $className Layout class name (e.g. "picture", "table", "text")
 * @property-read float $confidence Detection confidence score (0.0 to 1.0)
 * @property-read BoundingBox $boundingBox Bounding box in document coordinate space
 * @property-read float $areaFraction Fraction of page area covered (0.0 to 1.0)
 */
readonly class LayoutRegion
{
    public function __construct(
        public string $className,
        public float $confidence,
        public BoundingBox $boundingBox,
        public float $areaFraction,
    ) {
    }

    /**
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        /** @var array<string, mixed> $bboxData */
        $bboxData = isset($data['bounding_box']) && is_array($data['bounding_box'])
            ? $data['bounding_box']
            : [];

        return new self(
            className: is_string($data['class'] ?? null) ? $data['class'] : '',
            confidence: is_numeric($data['confidence'] ?? null) ? (float) $data['confidence'] : 0.0,
            boundingBox: BoundingBox::fromArray($bboxData),
            areaFraction: is_numeric($data['area_fraction'] ?? null) ? (float) $data['area_fraction'] : 0.0,
        );
    }
}
