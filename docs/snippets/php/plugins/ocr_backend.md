```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;

class CustomOcrBackend implements OcrBackend {
    private array $supportedLangs = ["eng", "deu", "fra"];

    public function name(): string {
        return "custom-ocr";
    }

    public function version(): string {
        return "1.0.0";
    }

    public function initialize(): void {
        // Load OCR model or initialize resources
    }

    public function shutdown(): void {
        // Cleanup OCR resources
    }

    public function processImage(string $imageBytes, object $config): object {
        // Process image bytes and return ExtractionResult
        // This would call your OCR engine (Tesseract, EasyOCR, etc.)
        return (object)[
            'content' => 'Extracted text from image',
            'mime_type' => 'image/png',
            'metadata' => ['ocr_engine' => 'custom-ocr'],
            'tables' => [],
            'detected_languages' => ['eng'],
        ];
    }

    public function processImageFile(string $path, object $config): object {
        // Read file and delegate to processImage
        $imageBytes = file_get_contents($path);
        return $this->processImage($imageBytes, $config);
    }

    public function supportsLanguage(string $lang): bool {
        return in_array($lang, $this->supportedLangs);
    }

    public function backendType(): string {
        return "OCREngine";
    }

    public function supportedLanguages(): array {
        return $this->supportedLangs;
    }

    public function supportsTableDetection(): bool {
        return true;
    }

    public function supportsDocumentProcessing(): bool {
        return false;
    }

    public function processDocument(string $path, object $config): object {
        throw new Exception("Document processing not supported");
    }
}

// Register the custom OCR backend
$backend = new CustomOcrBackend();
Kreuzberg::registerOcrBackend($backend);

echo "Custom OCR backend registered\n";
```
