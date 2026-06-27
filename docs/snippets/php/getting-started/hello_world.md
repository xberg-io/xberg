```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), \Xberg\ExtractionConfig::default());

$result = $resultOutput->results[0];
echo "Hello, " . substr($result->getContent(), 0, 50) . "\n";
```
