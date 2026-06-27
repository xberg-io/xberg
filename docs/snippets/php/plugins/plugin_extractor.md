```php title="PHP"
<?php declare(strict_types=1);

use Xberg\Xberg;

class CustomXmlExtractor implements DocumentExtractor {
    public function name(): string {
        return "custom-xml-extractor";
    }

    public function version(): string {
        return "1.0.0";
    }

    public function initialize(): void {
        // Initialize XML parser resources
    }

    public function shutdown(): void {
        // Cleanup resources
    }

    public function extract(string $content, string $mimeType, object $config): object {
        try {
            $xml = simplexml_load_string($content);
            $text = $this->extractTextFromXml($xml);

            return (object)[
                'content' => $text,
                'mime_type' => 'application/xml',
                'metadata' => [
                    'root_element' => $xml->getName(),
                    'extraction_method' => 'custom-xml-extractor'
                ],
                'tables' => [],
                'detected_languages' => null,
                'chunks' => null,
                'images' => null,
            ];
        } catch (Exception $e) {
            throw new Exception("XML parsing failed: " . $e->getMessage());
        }
    }

    public function supportedMimeTypes(): array {
        return [
            "application/xml",
            "text/xml",
            "application/xhtml+xml"
        ];
    }

    public function priority(): int {
        return 75;
    }

    private function extractTextFromXml($xml): string {
        $text = "";

        // Extract text from all elements
        foreach ($xml->children() as $child) {
            $childText = (string)$child;
            if (!empty(trim($childText))) {
                $text .= trim($childText) . "\n";
            }
        }

        return $text ?: (string)$xml;
    }
}

// Register the XML extractor
$extractor = new CustomXmlExtractor();
Xberg::registerDocumentExtractor($extractor);

echo "XML extractor registered\n";
```
