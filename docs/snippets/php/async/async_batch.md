```php title="PHP"
<?php

declare(strict_types=1);

use Xberg\Xberg;
use function Xberg\extract_batch_async;

$xberg = new Xberg();

// Async batch file extraction
$files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
$deferred = $xberg->extractBatchAsync($files);

// Do other work while extraction runs...
processOtherTasks();

// Block until all results are ready
$results = $deferred->getResults();

foreach ($results as $i => $result) {
    echo "{$files[$i]}: " . strlen($result->content) . " chars\n";
}

// With timeout
$deferred = $xberg->extractBatchAsync($files);
$results = $deferred->waitBatch(10000); // 10 second timeout

if ($results !== null) {
    foreach ($results as $result) {
        echo $result->content . "\n";
    }
} else {
    echo "Batch extraction timed out\n";
}

// Procedural API
$deferred = extract_batch_async($files);
$results = $deferred->getResults();
```
