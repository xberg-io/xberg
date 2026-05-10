```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\BatchFileItem;

$config = new ExtractionConfig();
$items = [
    new BatchFileItem('doc1.pdf'),
    new BatchFileItem('doc2.docx'),
    new BatchFileItem('report.pdf'),
];
$results = Kreuzberg::batchExtractFilesSync($items, $config);

foreach ($results as $i => $result) {
    echo "File $i: " . strlen($result->getContent()) . " chars\n";
}
```
