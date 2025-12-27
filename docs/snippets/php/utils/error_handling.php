```php
<?php

declare(strict_types=1);

/**
 * Comprehensive Error Handling
 *
 * Demonstrate proper error handling for document extraction operations.
 * Shows how to catch and handle different types of Kreuzberg exceptions.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Exceptions\ParsingException;
use Kreuzberg\Exceptions\OcrException;
use Kreuzberg\Exceptions\ValidationException;

$kreuzberg = new Kreuzberg();

try {
    $result = $kreuzberg->extractFile('document.pdf');
    echo "Extracted " . strlen($result->content) . " characters\n";
} catch (ParsingException $e) {
    echo "Failed to parse document: " . $e->getMessage() . "\n";
    echo "Error code: " . $e->getCode() . "\n";
} catch (OcrException $e) {
    echo "OCR processing failed: " . $e->getMessage() . "\n";
    echo "Suggestion: Check if document is scanned and OCR is properly configured\n";
} catch (KreuzbergException $e) {
    echo "Extraction error: " . $e->getMessage() . "\n";
    if ($e->getPrevious() !== null) {
        echo "Caused by: " . $e->getPrevious()->getMessage() . "\n";
    }
}

try {
    $config = new ExtractionConfig();
    $pdfBytes = file_get_contents('sample.pdf');

    if ($pdfBytes === false) {
        throw new \RuntimeException('Failed to read file');
    }

    $result = $kreuzberg->extractBytes($pdfBytes, 'application/pdf', $config);
    echo "Extracted from bytes: " . substr($result->content, 0, 100) . "...\n";
} catch (ValidationException $e) {
    echo "Invalid configuration or input: " . $e->getMessage() . "\n";
    echo "Details: " . $e->getFile() . " at line " . $e->getLine() . "\n";
} catch (OcrException $e) {
    echo "OCR failed: " . $e->getMessage() . "\n";
} catch (KreuzbergException $e) {
    echo "Extraction failed: " . $e->getMessage() . "\n";
} catch (\RuntimeException $e) {
    echo "File system error: " . $e->getMessage() . "\n";
}

$files = ['doc1.pdf', 'corrupted.pdf', 'doc3.docx'];
$successfulExtractions = [];
$failedExtractions = [];

foreach ($files as $file) {
    try {
        $result = $kreuzberg->extractFile($file);
        $successfulExtractions[$file] = $result;
        echo "Success: $file\n";
    } catch (KreuzbergException $e) {
        $failedExtractions[$file] = [
            'error' => $e->getMessage(),
            'type' => get_class($e),
        ];
        echo "Failed: $file - " . $e->getMessage() . "\n";
    }
}

echo "\nResults:\n";
echo "Successful: " . count($successfulExtractions) . "\n";
echo "Failed: " . count($failedExtractions) . "\n";

function extractWithRetry(
    Kreuzberg $kreuzberg,
    string $file,
    int $maxRetries = 3
): ?\Kreuzberg\Result\ExtractionResult {
    $attempt = 0;

    while ($attempt < $maxRetries) {
        try {
            return $kreuzberg->extractFile($file);
        } catch (OcrException $e) {
            $attempt++;
            if ($attempt >= $maxRetries) {
                echo "OCR failed after $maxRetries attempts: " . $e->getMessage() . "\n";
                return null;
            }
            echo "OCR attempt $attempt failed, retrying...\n";
            sleep(1);
        } catch (KreuzbergException $e) {
            echo "Fatal error (no retry): " . $e->getMessage() . "\n";
            return null;
        }
    }

    return null;
}

$result = extractWithRetry($kreuzberg, 'difficult_scan.pdf');
if ($result !== null) {
    echo "Successfully extracted with retry: " . strlen($result->content) . " chars\n";
}
```
