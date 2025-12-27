<?php

declare(strict_types=1);

/**
 * Error Handling Example
 *
 * Demonstrates comprehensive error handling for document extraction.
 * Shows how to handle various error scenarios gracefully.
 *
 * This example covers:
 * - File not found errors
 * - Invalid file format errors
 * - Corrupted file handling
 * - OCR errors
 * - Configuration errors
 * - Batch processing errors
 * - Try-catch patterns
 * - Error recovery strategies
 *
 * @package Kreuzberg
 */

require_once __DIR__ . '/../../packages/php/vendor/autoload.php';

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use Kreuzberg\Kreuzberg;
use function Kreuzberg\extract_bytes;
use function Kreuzberg\extract_file;


echo "=== Example 1: Basic Error Handling ===\n\n";

try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__DIR__ . '/nonexistent.pdf');

    echo "Content: {$result->content}\n";

} catch (KreuzbergException $e) {
    echo "Caught KreuzbergException:\n";
    echo "  Message: {$e->getMessage()}\n";
    echo "  Code: {$e->getCode()}\n";
    echo "  File: {$e->getFile()}\n";
    echo "  Line: {$e->getLine()}\n\n";
}


echo "=== Example 2: File Not Found Handling ===\n\n";

$files = [
    __DIR__ . '/../sample-documents/document.pdf',
    __DIR__ . '/missing_file.pdf',
    __DIR__ . '/../sample-documents/article.pdf',
];

foreach ($files as $file) {
    $filename = basename($file);

    try {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($file);
        echo "OK: {$filename} - " . strlen($result->content) . " characters\n";

    } catch (KreuzbergException $e) {
        echo "ERROR: {$filename} - {$e->getMessage()}\n";
    }
}

echo "\n";


echo "=== Example 3: Invalid File Format Handling ===\n\n";

try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__FILE__);

    echo "Extracted from PHP file: " . strlen($result->content) . " characters\n";
    echo "MIME type: {$result->mimeType}\n\n";

} catch (KreuzbergException $e) {
    echo "Error extracting from PHP file:\n";
    echo "  {$e->getMessage()}\n\n";
}


echo "=== Example 4: Corrupted File Handling ===\n\n";

try {
    $corruptedPath = sys_get_temp_dir() . '/corrupted.pdf';
    file_put_contents($corruptedPath, 'This is not a valid PDF file');

    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile($corruptedPath);

    echo "Content: {$result->content}\n";

    unlink($corruptedPath);

} catch (KreuzbergException $e) {
    echo "Caught error for corrupted file:\n";
    echo "  {$e->getMessage()}\n\n";

    if (file_exists(sys_get_temp_dir() . '/corrupted.pdf')) {
        unlink(sys_get_temp_dir() . '/corrupted.pdf');
    }
}


echo "=== Example 5: Invalid MIME Type Handling ===\n\n";

try {
    $data = file_get_contents(__DIR__ . '/../sample-documents/sample.pdf');
    if ($data === false) {
        throw new RuntimeException('Failed to read file');
    }

    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractBytes($data, 'application/invalid-mime-type');

    echo "Content: {$result->content}\n";

} catch (KreuzbergException | RuntimeException $e) {
    echo "Error with invalid MIME type:\n";
    echo "  {$e->getMessage()}\n\n";
}


echo "=== Example 6: OCR Configuration Errors ===\n\n";

try {
    $config = new ExtractionConfig(
        ocr: new OcrConfig(
            backend: 'invalid_backend',
            language: 'eng',
        ),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile(__DIR__ . '/../sample-documents/scanned.pdf');

    echo "Content: {$result->content}\n";

} catch (KreuzbergException $e) {
    echo "OCR configuration error:\n";
    echo "  {$e->getMessage()}\n\n";
}


echo "=== Example 7: Batch Processing Error Handling ===\n\n";

try {
    $files = [
        __DIR__ . '/../sample-documents/valid1.pdf',
        __DIR__ . '/invalid.pdf',
        __DIR__ . '/../sample-documents/valid2.pdf',
    ];

    $kreuzberg = new Kreuzberg();
    $results = $kreuzberg->batchExtractFiles($files);

    echo "All files processed successfully\n";

} catch (KreuzbergException $e) {
    echo "Batch processing failed:\n";
    echo "  {$e->getMessage()}\n";
    echo "\nNote: Batch operations fail fast on first error.\n";
    echo "Process files individually for better error handling:\n\n";

    foreach ($files as $file) {
        $filename = basename($file);
        try {
            $kreuzberg = new Kreuzberg();
            $result = $kreuzberg->extractFile($file);
            echo "  OK: {$filename}\n";
        } catch (KreuzbergException $e) {
            echo "  ERROR: {$filename} - {$e->getMessage()}\n";
        }
    }

    echo "\n";
}


echo "=== Example 8: Procedural API Error Handling ===\n\n";

try {
    $result = extract_file(__DIR__ . '/nonexistent.pdf');
    echo "Content: {$result->content}\n";

} catch (KreuzbergException $e) {
    echo "Procedural API error:\n";
    echo "  {$e->getMessage()}\n\n";
}


echo "=== Example 9: Graceful Degradation ===\n\n";

$file = __DIR__ . '/../sample-documents/sample.pdf';

try {
    $config = new ExtractionConfig(
        ocr: new OcrConfig(backend: 'tesseract', language: 'eng'),
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile($file);

    echo "Extracted with OCR: " . strlen($result->content) . " characters\n";

} catch (KreuzbergException $e) {
    echo "OCR failed ({$e->getMessage()}), falling back to basic extraction\n";

    try {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($file);

        echo "Extracted without OCR: " . strlen($result->content) . " characters\n";

    } catch (KreuzbergException $e2) {
        echo "Basic extraction also failed: {$e2->getMessage()}\n";
    }
}

echo "\n";


echo "=== Example 10: Error Recovery with Retries ===\n\n";

function extractWithRetry(string $file, int $maxRetries = 3): ?string
{
    $attempt = 0;

    while ($attempt < $maxRetries) {
        try {
            $kreuzberg = new Kreuzberg();
            $result = $kreuzberg->extractFile($file);
            return $result->content;

        } catch (KreuzbergException $e) {
            $attempt++;

            if ($attempt >= $maxRetries) {
                echo "Failed after {$maxRetries} attempts: {$e->getMessage()}\n";
                return null;
            }

            echo "Attempt {$attempt} failed, retrying... ({$e->getMessage()})\n";
            usleep(100000);
        }
    }

    return null;
}

$content = extractWithRetry(__DIR__ . '/nonexistent.pdf');

if ($content !== null) {
    echo "Extracted: " . strlen($content) . " characters\n";
} else {
    echo "Extraction failed after retries\n";
}

echo "\n";


echo "=== Example 11: Logging Errors ===\n\n";

function logError(string $message, array $context = []): void
{
    $timestamp = date('Y-m-d H:i:s');
    $logMessage = "[{$timestamp}] {$message}";

    if (!empty($context)) {
        $logMessage .= ' | Context: ' . json_encode($context);
    }

    echo $logMessage . "\n";
}

try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile(__DIR__ . '/missing.pdf');

} catch (KreuzbergException $e) {
    logError('Extraction failed', [
        'file' => __DIR__ . '/missing.pdf',
        'error' => $e->getMessage(),
        'code' => $e->getCode(),
        'trace' => $e->getTraceAsString(),
    ]);
}

echo "\n";


echo "=== Example 12: Custom Error Messages ===\n\n";

function safeExtract(string $file): array
{
    try {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($file);

        return [
            'success' => true,
            'content' => $result->content,
            'metadata' => $result->metadata,
            'error' => null,
        ];

    } catch (KreuzbergException $e) {
        $errorMessage = match (true) {
            str_contains($e->getMessage(), 'not found') => 'The file could not be found. Please check the file path.',
            str_contains($e->getMessage(), 'format') => 'The file format is not supported or the file is corrupted.',
            str_contains($e->getMessage(), 'permission') => 'Permission denied. Please check file permissions.',
            default => 'An error occurred while processing the file: ' . $e->getMessage(),
        };

        return [
            'success' => false,
            'content' => null,
            'metadata' => null,
            'error' => $errorMessage,
        ];
    }
}

$result = safeExtract(__DIR__ . '/missing.pdf');

if ($result['success']) {
    echo "Successfully extracted: " . strlen($result['content']) . " characters\n";
} else {
    echo "Extraction failed:\n";
    echo "  {$result['error']}\n";
}

echo "\n";


echo "=== Example 13: Validating Files Before Extraction ===\n\n";

function validateAndExtract(string $file): ?string
{
    if (!file_exists($file)) {
        echo "Error: File does not exist: {$file}\n";
        return null;
    }

    if (!is_readable($file)) {
        echo "Error: File is not readable: {$file}\n";
        return null;
    }

    $maxSize = 100 * 1024 * 1024;
    $fileSize = filesize($file);

    if ($fileSize === false) {
        echo "Error: Could not determine file size: {$file}\n";
        return null;
    }

    if ($fileSize > $maxSize) {
        echo "Error: File too large (" . round($fileSize / 1024 / 1024, 2) . " MB). Max: 100 MB\n";
        return null;
    }

    $allowedExtensions = ['pdf', 'docx', 'txt', 'html', 'xlsx', 'pptx'];
    $extension = strtolower(pathinfo($file, PATHINFO_EXTENSION));

    if (!in_array($extension, $allowedExtensions, true)) {
        echo "Error: File extension '.{$extension}' is not supported\n";
        return null;
    }

    try {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($file);
        return $result->content;

    } catch (KreuzbergException $e) {
        echo "Error during extraction: {$e->getMessage()}\n";
        return null;
    }
}

$content = validateAndExtract(__DIR__ . '/nonexistent.pdf');

if ($content !== null) {
    echo "Successfully extracted: " . strlen($content) . " characters\n";
}

echo "\n";


echo "=== Example 14: Error Handling Best Practices ===\n\n";

echo "Best practices for error handling:\n\n";

echo "1. Always wrap extraction calls in try-catch blocks\n";
echo "   - Catch KreuzbergException specifically\n";
echo "   - Log errors with context information\n\n";

echo "2. Validate inputs before processing\n";
echo "   - Check file existence and readability\n";
echo "   - Validate file size and format\n";
echo "   - Sanitize file paths\n\n";

echo "3. Provide meaningful error messages\n";
echo "   - Translate technical errors to user-friendly messages\n";
echo "   - Include actionable information\n\n";

echo "4. Implement graceful degradation\n";
echo "   - Fall back to simpler configurations on error\n";
echo "   - Retry with different settings\n\n";

echo "5. For batch processing\n";
echo "   - Process files individually for better error isolation\n";
echo "   - Continue processing even if some files fail\n";
echo "   - Collect and report all errors\n\n";

echo "6. Use proper logging\n";
echo "   - Log errors with timestamps and context\n";
echo "   - Use structured logging for better analysis\n";
echo "   - Consider different log levels (error, warning, info)\n\n";

echo "7. Handle edge cases\n";
echo "   - Empty files\n";
echo "   - Corrupted files\n";
echo "   - Unsupported formats\n";
echo "   - Permission issues\n\n";

echo "Done!\n";
