```php title="metadata.php"
<?php

declare(strict_types=1);

/**
 * Document Metadata Access
 *
 * Extract and access metadata from different document types including
 * PDFs, HTML, and other formats.
 */

require_once __DIR__ . '/vendor/autoload.php';


$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

if ($result->metadata?->pdf !== null) {
    $pdfMeta = $result->metadata->pdf;
    echo "Pages: " . ($pdfMeta['page_count'] ?? 'N/A') . "\n";
    echo "Author: " . ($pdfMeta['author'] ?? 'N/A') . "\n";
    echo "Title: " . ($pdfMeta['title'] ?? 'N/A') . "\n";
}

$htmlResult = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('page.html'), $config ?? \Xberg\ExtractionConfig::default())->results[0];

if (isset($htmlResult->metadata->html)) {
    $htmlMeta = $htmlResult->metadata->html;
    echo "Title: " . ($htmlMeta['title'] ?? 'N/A') . "\n";
    echo "Description: " . ($htmlMeta['description'] ?? 'N/A') . "\n";
}
```
