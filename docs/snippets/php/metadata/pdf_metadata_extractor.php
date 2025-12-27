```php
<?php

declare(strict_types=1);

/**
 * PDF Metadata Extractor Post-Processor
 *
 * Custom post-processor that extracts and enriches PDF metadata
 * during the extraction pipeline.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\PostProcessor\PostProcessorInterface;
use Kreuzberg\Types\ExtractionResult;
use Kreuzberg\Kreuzberg;

/**
 * Post-processor for extracting and enriching PDF metadata
 */
readonly class PdfMetadataExtractor implements PostProcessorInterface
{
    private int $processedCount;

    public function __construct()
    {
        $this->processedCount = 0;
    }

    /**
     * Get the name of this post-processor
     */
    public function name(): string
    {
        return 'pdf_metadata_extractor';
    }

    /**
     * Get the version of this post-processor
     */
    public function version(): string
    {
        return '1.0.0';
    }

    /**
     * Get the description of this post-processor
     */
    public function description(): string
    {
        return 'Extracts and enriches PDF metadata';
    }

    /**
     * Get the processing stage (early, normal, or late)
     */
    public function processingStage(): string
    {
        return 'early';
    }

    /**
     * Determine if this processor should handle the result
     */
    public function shouldProcess(ExtractionResult $result): bool
    {
        return $result->mimeType === 'application/pdf';
    }

    /**
     * Process the extraction result
     */
    public function process(ExtractionResult $result): ExtractionResult
    {
        $this->processedCount++;

        if (!isset($result->metadata->custom)) {
            $result->metadata->custom = [];
        }
        $result->metadata->custom['pdf_processed'] = true;
        $result->metadata->custom['processor_version'] = $this->version();

        return $result;
    }

    /**
     * Initialize the post-processor
     */
    public function initialize(): void
    {
        error_log("PDF metadata extractor initialized");
    }

    /**
     * Shutdown the post-processor
     */
    public function shutdown(): void
    {
        error_log("Processed {$this->processedCount} PDFs");
    }

    /**
     * Get the number of processed documents
     */
    public function getProcessedCount(): int
    {
        return $this->processedCount;
    }
}

$processor = new PdfMetadataExtractor();
Kreuzberg::registerPostProcessor($processor);
```
