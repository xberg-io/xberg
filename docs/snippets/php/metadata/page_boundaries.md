```php title="PHP"
<?php declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\PageConfig;

$config = ExtractionConfig::default();
$config->pages = new PageConfig(
    extractPages: true,
    insertPageMarkers: true,
    markerFormat: "\n\n=== PAGE {page_num} ===\n\n"
);

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri("document.pdf"), $config);

$result = $resultOutput->results[0];

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
