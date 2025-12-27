```php
<?php

declare(strict_types=1);

/**
 * Error Handling
 *
 * Robust error handling for document extraction operations.
 * Handle failures gracefully and implement retry strategies.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Exceptions\KreuzbergException;
use function Kreuzberg\extract_file;

try {
    $result = extract_file('document.pdf');
    echo "Extraction successful!\n";
    echo "Content length: " . strlen($result->content) . "\n";
} catch (KreuzbergException $e) {
    echo "Error: " . $e->getMessage() . "\n";
    echo "Code: " . $e->getCode() . "\n";
    error_log("Kreuzberg extraction failed: " . $e->getMessage());
}

function safeExtract(string $filePath): ?string
{
    if (!file_exists($filePath)) {
        error_log("File not found: $filePath");
        return null;
    }

    if (!is_readable($filePath)) {
        error_log("File not readable: $filePath");
        return null;
    }

    try {
        $result = extract_file($filePath);
        return $result->content;
    } catch (KreuzbergException $e) {
        error_log("Extraction error for $filePath: " . $e->getMessage());
        return null;
    }
}

$content = safeExtract('document.pdf');
if ($content !== null) {
    echo "Successfully extracted document\n";
} else {
    echo "Failed to extract document\n";
}

function extractWithRetry(
    string $filePath,
    int $maxRetries = 3,
    int $initialDelay = 1000
): ?string {
    $attempt = 0;
    $delay = $initialDelay;

    while ($attempt < $maxRetries) {
        try {
            $result = extract_file($filePath);
            return $result->content;
        } catch (KreuzbergException $e) {
            $attempt++;
            if ($attempt >= $maxRetries) {
                error_log("Max retries exceeded for $filePath: " . $e->getMessage());
                return null;
            }

            echo "Attempt $attempt failed, retrying in {$delay}ms...\n";
            usleep($delay * 1000);
            $delay *= 2; 
        }
    }

    return null;
}

$content = extractWithRetry('potentially_corrupt.pdf');
if ($content !== null) {
    echo "Document extracted after retry\n";
}

function validateExtractionResult(string $filePath): bool
{
    try {
        $result = extract_file($filePath);

        if (empty($result->content)) {
            error_log("Empty content extracted from $filePath");
            return false;
        }

        $minExpectedChars = 100;
        if (strlen($result->content) < $minExpectedChars) {
            error_log("Content too short from $filePath: " . strlen($result->content) . " chars");
            return false;
        }

        $nonPrintableRatio = (strlen($result->content) - strlen(preg_replace('/[^\x20-\x7E\x0A\x0D]/', '', $result->content))) / strlen($result->content);
        if ($nonPrintableRatio > 0.5) {
            error_log("High non-printable character ratio in $filePath");
            return false;
        }

        return true;
    } catch (KreuzbergException $e) {
        error_log("Validation failed for $filePath: " . $e->getMessage());
        return false;
    }
}

if (validateExtractionResult('document.pdf')) {
    echo "Extraction result validated successfully\n";
} else {
    echo "Extraction result validation failed\n";
}

$files = glob('documents/*.pdf');
$successful = [];
$failed = [];

foreach ($files as $file) {
    try {
        $result = extract_file($file);
        $successful[] = [
            'file' => $file,
            'content_length' => strlen($result->content),
            'tables' => count($result->tables),
        ];
    } catch (KreuzbergException $e) {
        $failed[] = [
            'file' => $file,
            'error' => $e->getMessage(),
            'code' => $e->getCode(),
        ];
    }
}

echo "\nBatch Processing Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Successful: " . count($successful) . "\n";
echo "Failed: " . count($failed) . "\n\n";

if (!empty($failed)) {
    echo "Failed files:\n";
    foreach ($failed as $failure) {
        echo "  - {$failure['file']}: {$failure['error']}\n";
    }
}

function extractWithFallback(string $filePath): ?string
{
    try {
        $result = extract_file($filePath);
        if (!empty($result->content)) {
            return $result->content;
        }
    } catch (KreuzbergException $e) {
        echo "Normal extraction failed, trying fallback strategies...\n";
    }

    try {
        $config = new ExtractionConfig(
            ocr: new \Kreuzberg\Config\OcrConfig(
                backend: 'tesseract',
                language: 'eng'
            )
        );
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($filePath);
        if (!empty($result->content)) {
            echo "Fallback: OCR extraction succeeded\n";
            return $result->content;
        }
    } catch (KreuzbergException $e) {
        echo "OCR fallback failed: " . $e->getMessage() . "\n";
    }

    try {
        $content = file_get_contents($filePath);
        if (!empty($content)) {
            echo "Fallback: Reading as plain text\n";
            return $content;
        }
    } catch (\Exception $e) {
        echo "Plain text fallback failed: " . $e->getMessage() . "\n";
    }

    return null;
}

$content = extractWithFallback('problematic_file.pdf');
if ($content !== null) {
    echo "Successfully extracted with fallback\n";
}

function extractWithTimeout(string $filePath, int $timeoutSeconds = 30): ?string
{
    $startTime = time();

    try {
        set_time_limit($timeoutSeconds);

        $result = extract_file($filePath);
        $elapsed = time() - $startTime;

        if ($elapsed > $timeoutSeconds) {
            error_log("Extraction exceeded timeout for $filePath");
            return null;
        }

        return $result->content;
    } catch (KreuzbergException $e) {
        error_log("Extraction error: " . $e->getMessage());
        return null;
    } finally {
        set_time_limit(0); 
    }
}

class DocumentExtractionException extends \Exception
{
    public function __construct(
        string $message,
        public readonly string $filePath,
        public readonly ?string $mimeType = null,
        ?\Throwable $previous = null
    ) {
        parent::__construct($message, 0, $previous);
    }
}

function extractOrThrow(string $filePath): string
{
    try {
        $result = extract_file($filePath);

        if (empty($result->content)) {
            throw new DocumentExtractionException(
                "No content extracted",
                $filePath,
                $result->mimeType
            );
        }

        return $result->content;
    } catch (KreuzbergException $e) {
        throw new DocumentExtractionException(
            "Extraction failed: " . $e->getMessage(),
            $filePath,
            previous: $e
        );
    }
}

try {
    $content = extractOrThrow('document.pdf');
    echo "Content: " . substr($content, 0, 100) . "...\n";
} catch (DocumentExtractionException $e) {
    echo "Failed to extract {$e->filePath}\n";
    echo "Reason: {$e->getMessage()}\n";
    if ($e->mimeType) {
        echo "MIME type: {$e->mimeType}\n";
    }
}

class LoggingKreuzberg
{
    public function __construct(
        private Kreuzberg $kreuzberg,
        private \Psr\Log\LoggerInterface $logger
    ) {}

    public function extractFile(string $filePath, ?string $mimeType = null): ?\Kreuzberg\Types\ExtractionResult
    {
        $this->logger->info("Starting extraction", ['file' => $filePath]);
        $startTime = microtime(true);

        try {
            $result = $this->kreuzberg->extractFile($filePath, $mimeType);
            $elapsed = microtime(true) - $startTime;

            $this->logger->info("Extraction successful", [
                'file' => $filePath,
                'duration' => $elapsed,
                'content_length' => strlen($result->content),
                'tables' => count($result->tables),
            ]);

            return $result;
        } catch (KreuzbergException $e) {
            $elapsed = microtime(true) - $startTime;

            $this->logger->error("Extraction failed", [
                'file' => $filePath,
                'duration' => $elapsed,
                'error' => $e->getMessage(),
                'code' => $e->getCode(),
            ]);

            return null;
        }
    }
}
```
