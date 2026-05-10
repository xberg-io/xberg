<!-- snippet:syntax-only -->
```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;

// PHP does not have native async/await. The ext-php-rs binding blocks internally
// using tokio::task::block_on. For concurrent operations, use batchExtractBytesSync
// or batchExtractBytesAsync with multiple items instead.

$content = file_get_contents('document.pdf');
$config = new ExtractionConfig();
// Note: This is labeled "async" in the API but blocks in PHP like the sync version
$result = Kreuzberg::extractBytesAsync($content, 'application/pdf', $config);

echo $result->getContent();
echo 'Tables: ' . count($result->getTables()) . "\n";
```
