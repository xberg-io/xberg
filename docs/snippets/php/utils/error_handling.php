```php title="error_handling.php"
<?php

declare(strict_types=1);

/**
 * Comprehensive Error Handling
 *
 * Demonstrate proper error handling for document extraction operations.
 * Shows how to catch and handle different types of Xberg exceptions.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Exceptions\XbergException;
use Xberg\Exceptions\ParsingException;
use Xberg\Exceptions\OcrException;
use Xberg\Exceptions\ValidationException;

$xberg = new Xberg();

try {
    $result = $xberg->extract('document.pdf');
    echo "Extracted " . strlen($result->content) . " characters\n";
} catch (ParsingException $e) {
    echo "Failed to parse document: " . $e->getMessage() . "\n";
    echo "Error code: " . $e->getCode() . "\n";
} catch (OcrException $e) {
    echo "OCR processing failed: " . $e->getMessage() . "\n";
    echo "Suggestion: Check if document is scanned and OCR is properly configured\n";
} catch (XbergException $e) {
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

    $result = $xberg->extract($pdfBytes, 'application/pdf', $config);
    echo "Extracted from bytes: " . substr($result->content, 0, 100) . "...\n";
} catch (ValidationException $e) {
    echo "Invalid configuration or input: " . $e->getMessage() . "\n";
    echo "Details: " . $e->getFile() . " at line " . $e->getLine() . "\n";
} catch (OcrException $e) {
    echo "OCR failed: " . $e->getMessage() . "\n";
} catch (XbergException $e) {
    echo "Extraction failed: " . $e->getMessage() . "\n";
} catch (\RuntimeException $e) {
    echo "File system error: " . $e->getMessage() . "\n";
}

$files = ['doc1.pdf', 'corrupted.pdf', 'doc3.docx'];
$successfulExtractions = [];
$failedExtractions = [];

foreach ($files as $file) {
    try {
        $result = $xberg->extract($file);
        $successfulExtractions[$file] = $result;
        echo "Success: $file\n";
    } catch (XbergException $e) {
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
    Xberg $xberg,
    string $file,
    int $maxRetries = 3
): ?\Xberg\Result\ExtractionResult {
    $attempt = 0;

    while ($attempt < $maxRetries) {
        try {
            return $xberg->extract($file);
        } catch (OcrException $e) {
            $attempt++;
            if ($attempt >= $maxRetries) {
                echo "OCR failed after $maxRetries attempts: " . $e->getMessage() . "\n";
                return null;
            }
            echo "OCR attempt $attempt failed, retrying...\n";
            sleep(1);
        } catch (XbergException $e) {
            echo "Fatal error (no retry): " . $e->getMessage() . "\n";
            return null;
        }
    }

    return null;
}

$result = extractWithRetry($xberg, 'difficult_scan.pdf');
if ($result !== null) {
    echo "Successfully extracted with retry: " . strlen($result->content) . " chars\n";
}
```
