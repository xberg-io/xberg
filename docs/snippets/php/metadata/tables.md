```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;

$result = Kreuzberg::extract_file_sync("document.pdf", null, new ExtractionConfig());

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
