```php title="PHP"
<?php declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri("document.pdf"), \Xberg\ExtractionConfig::default());

$result = $resultOutput->results[0];

foreach ($result->tables as $table) {
    echo "Table on page " . $table->page_number . " with " . count($table->cells) . " rows\n";
    echo "Markdown representation:\n";
    echo $table->markdown . "\n";

    // Access cell data
    foreach ($table->cells as $rowIndex => $row) {
        foreach ($row as $colIndex => $cellContent) {
            echo "Cell[$rowIndex][$colIndex]: $cellContent\n";
        }
    }
}
?>
```
