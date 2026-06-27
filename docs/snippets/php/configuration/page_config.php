```php title="page_config.php"
<?php

declare(strict_types=1);

/**
 * PageConfig - Page-Level Extraction
 *
 * Configure per-page content extraction and page markers for maintaining
 * document structure in the extracted text.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\PageConfig;

$config = new ExtractionConfig(
    page: new PageConfig(
        extractPages: false,
        insertPageMarkers: true,
        markerFormat: '--- Page {page_number} ---'
    )
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('report.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Content with page markers:\n";
echo str_repeat('=', 60) . "\n";
echo $result->getContent() . "\n\n";

$pageConfig = new ExtractionConfig(
    page: new PageConfig(
        extractPages: true,
        insertPageMarkers: false
    )
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('multi_page.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

foreach ($result->pages ?? [] as $page) {
    echo "Page {$page->pageNumber}:\n";
    echo str_repeat('-', 60) . "\n";
    echo substr($page->content, 0, 200) . "...\n";
    echo "Tables on page: " . count($page->tables) . "\n";
    echo "Images on page: " . count($page->images) . "\n\n";
}

$customConfig = new ExtractionConfig(
    page: new PageConfig(
        extractPages: false,
        insertPageMarkers: true,
        markerFormat: "\n\n========== PAGE {page_number} ==========\n\n"
    )
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

$pages = preg_split('/={10} PAGE \d+ ={10}/', $result->getContent());
echo "Split into " . count($pages) . " sections\n";

$allPagesConfig = new ExtractionConfig(
    page: new PageConfig(extractPages: true)
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('large_doc.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

$selectedPages = array_filter(
    $result->pages ?? [],
    fn($page) => $page->pageNumber >= 10 && $page->pageNumber <= 20
);

echo "\nSelected pages 10-20:\n";
foreach ($selectedPages as $page) {
    echo "Page {$page->pageNumber}: " . strlen($page->content) . " chars\n";
}
```
