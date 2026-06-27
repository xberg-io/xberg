```php title="error_handling.php"
<?php

declare(strict_types=1);

/**
 * Error Handling
 *
 * Robust error handling for document extraction operations.
 * Handle failures gracefully and implement retry strategies.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Exceptions\XbergException;

try {
    $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];
    echo "Extraction successful!\n";
    echo "Content length: " . strlen($result->getContent()) . "\n";
} catch (XbergException $e) {
    echo "Error: " . $e->getMessage() . "\n";
    echo "Code: " . $e->getCode() . "\n";
    error_log("Xberg extraction failed: " . $e->getMessage());
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
        $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($filePath), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];
        return $result->getContent();
    } catch (XbergException $e) {
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
            $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($filePath), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];
            return $result->getContent();
        } catch (XbergException $e) {
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
        $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($filePath), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

        if (empty($result->getContent())) {
            error_log("Empty content extracted from $filePath");
            return false;
        }

        $minExpectedChars = 100;
        if (strlen($result->getContent()) < $minExpectedChars) {
            error_log("Content too short from $filePath: " . strlen($result->getContent()) . " chars");
            return false;
        }

        $nonPrintableRatio = (strlen($result->getContent()) - strlen(preg_replace('/[^\x20-\x7E\x0A\x0D]/', '', $result->getContent()))) / strlen($result->getContent());
        if ($nonPrintableRatio > 0.5) {
            error_log("High non-printable character ratio in $filePath");
            return false;
        }

        return true;
    } catch (XbergException $e) {
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
        $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($file), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];
        $successful[] = [
            'file' => $file,
            'content_length' => strlen($result->getContent()),
            'tables' => count($result->tables),
        ];
    } catch (XbergException $e) {
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
        $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($filePath), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];
        if (!empty($result->getContent())) {
            return $result->getContent();
        }
    } catch (XbergException $e) {
        echo "Normal extraction failed, trying fallback strategies...\n";
    }

    try {
        $config = new ExtractionConfig(
            ocr: new \Xberg\Config\OcrConfig(
                backend: 'tesseract',
                language: 'eng'
            )
        );
        $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($filePath), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];
        if (!empty($result->getContent())) {
            echo "Fallback: OCR extraction succeeded\n";
            return $result->getContent();
        }
    } catch (XbergException $e) {
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

        $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($filePath), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];
        $elapsed = time() - $startTime;

        if ($elapsed > $timeoutSeconds) {
            error_log("Extraction exceeded timeout for $filePath");
            return null;
        }

        return $result->getContent();
    } catch (XbergException $e) {
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
        $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri($filePath), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

        if (empty($result->getContent())) {
            throw new DocumentExtractionException(
                "No content extracted",
                $filePath,
                $result->mimeType
            );
        }

        return $result->getContent();
    } catch (XbergException $e) {
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

class LoggingXberg
{
    public function __construct(
        private Xberg $xberg,
        private \Psr\Log\LoggerInterface $logger
    ) {}

    public function extract(string $filePath, ?string $mimeType = null): ?\Xberg\Types\ExtractionResult
    {
        $this->logger->info("Starting extraction", ['file' => $filePath]);
        $startTime = microtime(true);

        try {
            $result = $this->xberg->extract($filePath, $mimeType);
            $elapsed = microtime(true) - $startTime;

            $this->logger->info("Extraction successful", [
                'file' => $filePath,
                'duration' => $elapsed,
                'content_length' => strlen($result->getContent()),
                'tables' => count($result->tables),
            ]);

            return $result;
        } catch (XbergException $e) {
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
