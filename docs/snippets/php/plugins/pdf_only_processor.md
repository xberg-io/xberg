```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;

class PdfOnlyProcessor implements PostProcessor {
    public function name(): string {
        return "pdf-only-processor";
    }

    public function version(): string {
        return "1.0.0";
    }

    public function initialize(): void {
        // Initialize PDF-specific resources
    }

    public function shutdown(): void {
        // Cleanup resources
    }

    public function process(object &$result, object $config): void {
        // Only execute for PDFs
        if ($result->mime_type !== 'application/pdf') {
            return;
        }
        
        // Process PDF-specific logic
        // For example: extract page information, count pages, extract images, etc.
        
        if (!isset($result->metadata)) {
            $result->metadata = [];
        }
        
        if (is_array($result->metadata)) {
            $result->metadata['pdf_processed'] = true;
            $result->metadata['processor_version'] = '1.0.0';
        }
    }

    public function processingStage(): string {
        return "Middle";
    }

    public function shouldProcess(object $result, object $config): bool {
        // Only process PDFs with content
        return $result->mime_type === 'application/pdf' && !empty($result->content);
    }

    public function estimatedDurationMs(object $result): int {
        // PDF processing varies by size
        return 50;
    }

    public function priority(): int {
        return 75;
    }
}

// Register the PDF-only processor
$processor = new PdfOnlyProcessor();
Kreuzberg::registerPostProcessor($processor);

echo "PDF-only processor registered\n";
```
