```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;

$config = new ExtractionConfig();
$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo "Content: " . $result->getContent() . "\n";
echo "MIME Type: " . $result->getMimeType() . "\n";
echo "Tables: " . count($result->getTables()) . "\n";
```
