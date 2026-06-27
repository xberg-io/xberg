```php title="PHP (Amp v3+)"
<?php

declare(strict_types=1);

// Requires: composer require amphp/amp ^3.0
use Xberg\Xberg;
use Xberg\Async\AmpBridge;

$xberg = new Xberg();

// Single file extraction with Amp Future
$deferred = $xberg->extractAsync('document.pdf');
$future = AmpBridge::toFuture($deferred);
$result = $future->await();
echo $result->content;

// Batch extraction with Amp Future
$files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
$batchDeferred = $xberg->extractBatchAsync($files);
$batchFuture = AmpBridge::toBatchFuture($batchDeferred);
$results = $batchFuture->await();

foreach ($results as $i => $result) {
    echo "{$files[$i]}: {$result->content}\n";
}
```
