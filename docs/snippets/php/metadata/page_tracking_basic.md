```php title="PHP"
<?php declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\PageConfig;

$config = ExtractionConfig::default();
$config->pages = new PageConfig(
    extractPages: true,
    insertPageMarkers: false,
    markerFormat: "\n\n<!-- PAGE {page_num} -->\n\n"
);

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri("document.pdf"), $config);

$result = $resultOutput->results[0];

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
