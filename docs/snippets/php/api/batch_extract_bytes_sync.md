```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\BatchBytesItem;

$config = new ExtractionConfig();
$items = [
    new BatchBytesItem('Hello, world!', 'text/plain'),
    new BatchBytesItem("# Heading\n\nParagraph text.", 'text/markdown'),
];
$results = Kreuzberg::batchExtractBytesSync($items, $config);

foreach ($results as $i => $result) {
    echo "Item $i: " . strlen($result->getContent()) . " chars\n";
}
```
