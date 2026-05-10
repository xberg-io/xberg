```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;

class WordCountProcessor implements PostProcessor {
    public function name(): string {
        return "word-count";
    }

    public function version(): string {
        return "1.0.0";
    }

    public function initialize(): void {
        // Initialize word counting resources
    }

    public function shutdown(): void {
        // Cleanup resources
    }

    public function process(object &$result, object $config): void {
        $wordCount = count(preg_split('/\s+/', trim($result->content), -1, PREG_SPLIT_NO_EMPTY));
        
        // Add word count to metadata
        if (!isset($result->metadata)) {
            $result->metadata = [];
        }
        
        if (is_array($result->metadata)) {
            $result->metadata['word_count'] = $wordCount;
        } else {
            $result->metadata = (array)$result->metadata;
            $result->metadata['word_count'] = $wordCount;
        }
    }

    public function processingStage(): string {
        return "Early";
    }

    public function shouldProcess(object $result, object $config): bool {
        // Only process if content is not empty
        return !empty($result->content);
    }

    public function estimatedDurationMs(object $result): int {
        // Word counting is very fast
        return 1;
    }

    public function priority(): int {
        return 50;
    }
}

// Register the word-count post-processor
$processor = new WordCountProcessor();
Kreuzberg::registerPostProcessor($processor);

echo "Word-count processor registered\n";
```
