```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;

$config = new ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true
);

$result = Xberg::extractSync('document.pdf', null, $config);

echo $result->getContent();
?>
```
