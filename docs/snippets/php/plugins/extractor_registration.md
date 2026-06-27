```php title="PHP"
<?php declare(strict_types=1);

use Xberg\Xberg;

class CustomJsonExtractor implements DocumentExtractor {
    public function name(): string {
        return "custom-json-extractor";
    }

    public function version(): string {
        return "1.0.0";
    }

    public function initialize(): void {
        // Initialize resources
    }

    public function shutdown(): void {
        // Cleanup resources
    }

    public function extract(string $content, string $mimeType, object $config): object {
        $json = json_decode($content, true);
        $text = $this->extractTextFromJson($json);

        return (object)[
            'content' => $text,
            'mime_type' => 'application/json',
            'metadata' => [],
            'tables' => [],
            'detected_languages' => null,
            'chunks' => null,
            'images' => null,
        ];
    }

    public function supportedMimeTypes(): array {
        return ["application/json", "text/json"];
    }

    public function priority(): int {
        return 50;
    }

    private function extractTextFromJson($value): string {
        if (is_string($value)) {
            return "$value\n";
        }
        if (is_array($value)) {
            $result = "";
            foreach ($value as $item) {
                $result .= $this->extractTextFromJson($item);
            }
            return $result;
        }
        return "";
    }
}

// Register the custom extractor
// Note: Document extractor registration would use a similar pattern
// when the binding API is available
$extractor = new CustomJsonExtractor();
Xberg::registerDocumentExtractor($extractor);
```
