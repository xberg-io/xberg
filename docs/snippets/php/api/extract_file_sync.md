```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;

$config = new ExtractionConfig();
$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo $result->getContent();
echo 'MIME type: ' . $result->getMimeType() . "\n";
echo 'Tables: ' . count($result->getTables()) . "\n";
```
