```php title="page_tracking_basic.php"
<?php

declare(strict_types=1);

/**
 * Basic Page Tracking
 *
 * Extract individual pages with their content, tables, and images
 * using page extraction configuration.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\PageConfig;

$config = new ExtractionConfig(
    pages: new PageConfig(
        extractPages: true
    )
);

$xberg = new Xberg($config);
$result = $xberg->extract('document.pdf');

if (!empty($result->pages)) {
    foreach ($result->pages as $page) {
        echo "Page {$page->pageNumber}:\n";
        echo "  Content: " . strlen($page->content) . " chars\n";
        echo "  Tables: " . count($page->tables) . "\n";
        echo "  Images: " . count($page->images) . "\n\n";
    }
}
```
