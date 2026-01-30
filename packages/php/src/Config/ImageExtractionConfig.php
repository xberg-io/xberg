<?php

declare(strict_types=1);

namespace Kreuzberg\Config;

/**
 * Image extraction configuration.
 */
readonly class ImageExtractionConfig
{
    public function __construct(
        /**
         * Enable image extraction from documents.
         *
         * When enabled, images are extracted from documents and saved/processed
         * separately from text content. Extracted images can be further analyzed
         * or stored for later retrieval.
         *
         * @var bool
         * @default true
         */
        public bool $extractImages = true,

        /**
         * Target DPI for image extraction.
         *
         * Controls the resolution at which images are extracted. Higher DPI
         * results in better quality but larger file sizes.
         *
         * @var int
         * @default 300
         */
        public int $targetDpi = 300,

        /**
         * Maximum image dimension (width or height).
         *
         * Images larger than this will be scaled down. Helps prevent
         * memory issues with very large images.
         *
         * @var int
         * @default 4096
         */
        public int $maxImageDimension = 4096,

        /**
         * Automatically adjust DPI based on image characteristics.
         *
         * When enabled, the extractor may adjust DPI up or down based on
         * the source image to optimize quality and performance.
         *
         * @var bool
         * @default true
         */
        public bool $autoAdjustDpi = true,

        /**
         * Minimum DPI for extraction.
         *
         * When auto-adjusting DPI, this is the lower bound.
         *
         * @var int
         * @default 72
         */
        public int $minDpi = 72,

        /**
         * Maximum DPI for extraction.
         *
         * When auto-adjusting DPI, this is the upper bound.
         *
         * @var int
         * @default 600
         */
        public int $maxDpi = 600,
    ) {
    }

    /**
     * Create configuration from array data.
     *
     * @param array<string, mixed> $data
     */
    public static function fromArray(array $data): self
    {
        /** @var bool $extractImages */
        $extractImages = $data['extract_images'] ?? true;
        if (!is_bool($extractImages)) {
            /** @var bool $extractImages */
            $extractImages = (bool) $extractImages;
        }

        /** @var int $targetDpi */
        $targetDpi = $data['target_dpi'] ?? 300;
        if (!is_int($targetDpi)) {
            /** @var int $targetDpi */
            $targetDpi = (int) $targetDpi;
        }

        /** @var int $maxImageDimension */
        $maxImageDimension = $data['max_image_dimension'] ?? 4096;
        if (!is_int($maxImageDimension)) {
            /** @var int $maxImageDimension */
            $maxImageDimension = (int) $maxImageDimension;
        }

        /** @var bool $autoAdjustDpi */
        $autoAdjustDpi = $data['auto_adjust_dpi'] ?? true;
        if (!is_bool($autoAdjustDpi)) {
            /** @var bool $autoAdjustDpi */
            $autoAdjustDpi = (bool) $autoAdjustDpi;
        }

        /** @var int $minDpi */
        $minDpi = $data['min_dpi'] ?? 72;
        if (!is_int($minDpi)) {
            /** @var int $minDpi */
            $minDpi = (int) $minDpi;
        }

        /** @var int $maxDpi */
        $maxDpi = $data['max_dpi'] ?? 600;
        if (!is_int($maxDpi)) {
            /** @var int $maxDpi */
            $maxDpi = (int) $maxDpi;
        }

        return new self(
            extractImages: $extractImages,
            targetDpi: $targetDpi,
            maxImageDimension: $maxImageDimension,
            autoAdjustDpi: $autoAdjustDpi,
            minDpi: $minDpi,
            maxDpi: $maxDpi,
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
        return [
            'extract_images' => $this->extractImages,
            'target_dpi' => $this->targetDpi,
            'max_image_dimension' => $this->maxImageDimension,
            'auto_adjust_dpi' => $this->autoAdjustDpi,
            'min_dpi' => $this->minDpi,
            'max_dpi' => $this->maxDpi,
        ];
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
