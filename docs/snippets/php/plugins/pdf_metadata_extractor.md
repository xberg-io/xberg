```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;

class PdfMetadataExtractor implements PostProcessor {
    public function name(): string {
        return "pdf-metadata-extractor";
    }

    public function version(): string {
        return "1.0.0";
    }

    public function initialize(): void {
        // Load PDF parsing libraries if needed
    }

    public function shutdown(): void {
        // Cleanup resources
    }

    public function process(object &$result, object $config): void {
        // Only process PDFs
        if ($result->mime_type !== 'application/pdf') {
            return;
        }
        
        // Extract and attach metadata
        if (!isset($result->metadata)) {
            $result->metadata = [];
        }
        
        if (is_array($result->metadata)) {
            $result->metadata = array_merge($result->metadata, [
                'pdf_processor' => 'pdf-metadata-extractor',
                'extracted_at' => date('Y-m-d H:i:s'),
            ]);
        }
    }

    public function processingStage(): string {
        return "Middle";
    }

    public function shouldProcess(object $result, object $config): bool {
        return $result->mime_type === 'application/pdf';
    }

    public function estimatedDurationMs(object $result): int {
        return 10;
    }

    public function priority(): int {
        return 60;
    }
}

// Register the PDF metadata extractor
$processor = new PdfMetadataExtractor();
Kreuzberg::registerPostProcessor($processor);

echo "PDF metadata extractor registered\n";
```
