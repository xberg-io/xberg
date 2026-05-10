```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;

class LoggingPostProcessor implements PostProcessor {
    public function name(): string {
        return "logging-processor";
    }

    public function version(): string {
        return "1.0.0";
    }

    public function initialize(): void {
        error_log("LoggingPostProcessor initialized");
    }

    public function shutdown(): void {
        error_log("LoggingPostProcessor shutting down");
    }

    public function process(object &$result, object $config): void {
        error_log("Processing: " . $result->mime_type);
        error_log("Content length: " . strlen($result->content));
        error_log("Metadata: " . json_encode($result->metadata));
    }

    public function processingStage(): string {
        return "Early";
    }

    public function shouldProcess(object $result, object $config): bool {
        // Only log non-empty results
        return !empty($result->content);
    }

    public function estimatedDurationMs(object $result): int {
        // Logging takes minimal time
        return 1;
    }

    public function priority(): int {
        return 10;
    }
}

// Register the logging post-processor
$processor = new LoggingPostProcessor();
Kreuzberg::registerPostProcessor($processor);

error_log("Logging post-processor registered");
```
