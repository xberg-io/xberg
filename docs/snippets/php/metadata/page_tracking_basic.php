```php
<?php

declare(strict_types=1);

/**
 * Basic Page Tracking
 *
 * Extract individual pages with their content, tables, and images
 * using page extraction configuration.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\PageConfig;

$config = new ExtractionConfig(
    pages: new PageConfig(
        extractPages: true
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('document.pdf');

if (!empty($result->pages)) {
    foreach ($result->pages as $page) {
        echo "Page {$page->pageNumber}:\n";
        echo "  Content: " . strlen($page->content) . " chars\n";
        echo "  Tables: " . count($page->tables) . "\n";
        echo "  Images: " . count($page->images) . "\n\n";
    }
}
```
