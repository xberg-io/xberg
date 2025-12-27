<?php

/**
 * Example: Custom OCR Backend Implementation
 *
 * This example demonstrates how to create and register a custom OCR backend
 * for use with Kreuzberg's extraction pipeline.
 */

require_once __DIR__ . '/../vendor/autoload.php';

use Kreuzberg\Plugins\OcrBackendInterface;
use Kreuzberg\Plugins\OcrBackendRegistry;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;

/**
 * Simple OCR backend that returns mock data.
 *
 * In a real implementation, you would integrate with an OCR library
 * such as Tesseract, EasyOCR, PaddleOCR, or a cloud service.
 */
class SimpleOcrBackend implements OcrBackendInterface
{
    public function process(string $imageData, string $language): array
    {

        $text = $this->performOcr($imageData, $language);

        return [
            'content' => $text,
            'metadata' => [
                'confidence' => 0.95,
                'processing_time_ms' => 120,
                'engine' => 'simple-ocr-v1',
                'language' => $language,
            ],
            'tables' => []
        ];
    }

    private function performOcr(string $imageData, string $language): string
    {
        return "This is mock OCR output for a {$language} image.";
    }
}

/**
 * Advanced OCR backend with table detection.
 *
 * This example shows how to return table data in the correct format.
 */
class AdvancedOcrBackend implements OcrBackendInterface
{
    public function process(string $imageData, string $language): array
    {
        $text = $this->performOcr($imageData, $language);
        $tables = $this->detectTables($imageData);

        return [
            'content' => $text,
            'metadata' => [
                'confidence' => 0.92,
                'has_tables' => count($tables) > 0,
                'table_count' => count($tables),
            ],
            'tables' => $tables
        ];
    }

    private function performOcr(string $imageData, string $language): string
    {
        return "Invoice\nDate: 2024-01-15\nAmount: $150.00";
    }

    private function detectTables(string $imageData): array
    {
        return [
            [
                'cells' => [
                    ['Item', 'Quantity', 'Price'],
                    ['Widget A', '2', '$50.00'],
                    ['Widget B', '1', '$50.00'],
                ],
                'markdown' => "| Item     | Quantity | Price  |\n|----------|----------|--------|\n| Widget A | 2        | $50.00 |\n| Widget B | 1        | $50.00 |",
                'page_number' => 1
            ]
        ];
    }
}

echo "=== Example 1: Simple OCR Backend ===\n";

$simpleBackend = new SimpleOcrBackend();
OcrBackendRegistry::register(
    'simple-ocr',
    [$simpleBackend, 'process'],
    ['eng', 'deu', 'fra']
);

$backends = OcrBackendRegistry::list();
echo "Registered backends: " . implode(', ', $backends) . "\n";

$config = new ExtractionConfig();
$config->ocr = new OcrConfig();
$config->ocr->backend = 'simple-ocr';
$config->ocr->language = 'eng';


OcrBackendRegistry::unregister('simple-ocr');

echo "\n";

echo "=== Example 2: Advanced OCR Backend with Tables ===\n";

$advancedBackend = new AdvancedOcrBackend();
OcrBackendRegistry::register(
    'advanced-ocr',
    [$advancedBackend, 'process'],
    ['eng', 'deu', 'fra', 'spa', 'ita']
);

$config->ocr->backend = 'advanced-ocr';


OcrBackendRegistry::unregister('advanced-ocr');

echo "\n";

echo "=== Example 3: Closure-based OCR Backend ===\n";

$ocrCallback = function(string $imageData, string $language): array {
    return [
        'content' => "Simple closure-based OCR result",
        'metadata' => ['type' => 'closure'],
        'tables' => []
    ];
};

OcrBackendRegistry::register('closure-ocr', $ocrCallback, ['eng']);

echo "Registered closure-based backend\n";

OcrBackendRegistry::unregister('closure-ocr');

echo "\n";

echo "=== Example 4: Error Handling ===\n";

class ErrorHandlingBackend implements OcrBackendInterface
{
    public function process(string $imageData, string $language): array
    {
        try {
            if (strlen($imageData) < 100) {
                throw new \RuntimeException('Image data too small');
            }

            $text = "Processed successfully";

            return [
                'content' => $text,
                'metadata' => [],
                'tables' => []
            ];
        } catch (\Exception $e) {
            error_log("OCR error: " . $e->getMessage());

            throw new \Exception("OCR processing failed: " . $e->getMessage());
        }
    }
}

$errorBackend = new ErrorHandlingBackend();
OcrBackendRegistry::register(
    'error-backend',
    [$errorBackend, 'process'],
    ['eng']
);

echo "Registered error-handling backend\n";


OcrBackendRegistry::unregister('error-backend');

echo "\n";
echo "All examples completed successfully!\n";
