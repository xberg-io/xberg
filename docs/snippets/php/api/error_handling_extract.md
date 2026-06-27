```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\XbergException;

function extract_text(string $bytes, string $mime_type): string {
    $config = new ExtractionConfig();
    $result = Xberg::extractSync($bytes, $mime_type, $config);
    return $result->getContent();
}

$bytes = file_get_contents('document.pdf') ?: '';
try {
    $text = extract_text($bytes, 'application/pdf');
    echo "Extracted " . strlen($text) . " chars\n";
} catch (XbergException $e) {
    // All Xberg errors are XbergException
    // Check the message for error type details
    $message = $e->getMessage();
    if (strpos($message, 'not supported') !== false) {
        echo "Format not supported\n";
    } elseif (strpos($message, 'OCR') !== false) {
        echo "OCR failed: " . $message . "\n";
    } else {
        echo "Error: " . $message . "\n";
    }
}
```
