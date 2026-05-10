```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;

// Discover configuration from file system
$config = ExtractionConfig::discover() ?? new ExtractionConfig();
$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo $result->getContent();
?>
```
