```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;

$config = new ExtractionConfig();
$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo "Content:\n";
echo $result->getContent();
```
