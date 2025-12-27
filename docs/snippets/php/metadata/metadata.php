```php
<?php

declare(strict_types=1);

/**
 * Document Metadata Access
 *
 * Extract and access metadata from different document types including
 * PDFs, HTML, and other formats.
 */

require_once __DIR__ . '/vendor/autoload.php';

use function Kreuzberg\extract_file;

$result = extract_file('document.pdf');

if (isset($result->metadata->pdf)) {
    $pdfMeta = $result->metadata->pdf;
    echo "Pages: " . ($pdfMeta['page_count'] ?? 'N/A') . "\n";
    echo "Author: " . ($pdfMeta['author'] ?? 'N/A') . "\n";
    echo "Title: " . ($pdfMeta['title'] ?? 'N/A') . "\n";
}

$htmlResult = extract_file('page.html');

if (isset($htmlResult->metadata->html)) {
    $htmlMeta = $htmlResult->metadata->html;
    echo "Title: " . ($htmlMeta['title'] ?? 'N/A') . "\n";
    echo "Description: " . ($htmlMeta['description'] ?? 'N/A') . "\n";
}
```
