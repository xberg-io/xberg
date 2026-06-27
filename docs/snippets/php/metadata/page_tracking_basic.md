```php title="PHP"
<?php declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\PageConfig;

$config = new ExtractionConfig();
$config->pages = new PageConfig(
    extractPages: true,
    insertPageMarkers: false,
    markerFormat: "\n\n<!-- PAGE {page_num} -->\n\n"
);

$result = Xberg::extract_sync("document.pdf", null, $config);

if ($result->pages !== null) {
    foreach ($result->pages as $page) {
        echo "Page " . $page->page_number . ":\n";
        echo "  Content: " . strlen($page->content) . " chars\n";
        echo "  Tables: " . count($page->tables ?? []) . "\n";
        echo "  Images: " . count($page->images ?? []) . "\n";
    }
}
?>
```
