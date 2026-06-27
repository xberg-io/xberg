```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;

// Discover configuration from file system
$config = ExtractionConfig::discover() ?? ExtractionConfig::default();
$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);
$result = $resultOutput->results[0];

echo $result->getContent();
?>
```
