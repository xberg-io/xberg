```php title="PHP"
<?php declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\PageConfig;

$config = new ExtractionConfig();
$config->pages = new PageConfig(
    extractPages: true,
    insertPageMarkers: true,
    markerFormat: "\n\n=== PAGE {page_num} ===\n\n"
);

$result = Xberg::extract_sync("document.pdf", null, $config);

// Content with inline page markers
echo "Full content with markers:\n";
echo $result->content . "\n\n";

// Or access pages separately with boundaries preserved
if ($result->pages !== null) {
    foreach ($result->pages as $page) {
        echo "--- Page " . $page->page_number . " (boundary) ---\n";
        echo $page->content . "\n";
    }
}
?>
```
