```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;

$content = file_get_contents('document.pdf');
$config = new ExtractionConfig();
$result = Kreuzberg::extractBytesSync($content, 'application/pdf', $config);

echo $result->getContent();
echo 'Tables: ' . count($result->getTables()) . "\n";
```
