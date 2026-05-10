<!-- snippet:syntax-only -->
```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;

// PHP does not have native async/await. The ext-php-rs binding blocks internally
// using tokio::task::block_on. This behaves like the sync version in PHP.

$config = new ExtractionConfig();
// Note: This is labeled "async" in the API but blocks in PHP like the sync version
$result = Kreuzberg::extractFileAsync('document.pdf', null, $config);

echo $result->getContent();
echo 'MIME type: ' . $result->getMimeType() . "\n";
echo 'Tables: ' . count($result->getTables()) . "\n";
```
