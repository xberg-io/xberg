```php
<?php

declare(strict_types=1);

/**
 * Page Boundary Tracking
 *
 * Access page boundary information to extract content from specific pages
 * using byte offsets in the extracted content.
 */

require_once __DIR__ . '/vendor/autoload.php';

use function Kreuzberg\extract_file;

$result = extract_file('document.pdf');

if (isset($result->metadata->pages->boundaries) && !empty($result->metadata->pages->boundaries)) {
    $boundaries = $result->metadata->pages->boundaries;
    $contentBytes = $result->content;

    $pagesToShow = array_slice($boundaries, 0, 3);

    foreach ($pagesToShow as $boundary) {
        $pageContent = mb_substr(
            $contentBytes,
            $boundary->byteStart,
            $boundary->byteEnd - $boundary->byteStart
        );

        echo "Page {$boundary->pageNumber}:\n";
        echo "  Byte range: {$boundary->byteStart}-{$boundary->byteEnd}\n";
        echo "  Preview: " . mb_substr($pageContent, 0, 100) . "...\n\n";
    }
}
```
