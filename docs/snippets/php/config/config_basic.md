```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;

$config = new ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true
);

$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo $result->getContent();
?>
```
