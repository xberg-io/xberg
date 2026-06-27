```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\TokenReductionOptions;

$config = new ExtractionConfig(
    tokenReduction: new TokenReductionOptions(
        mode: 'moderate',
        preserveImportantWords: true
    )
);

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);

$result = $resultOutput->results[0];

echo "Reduced content: " . substr($result->getContent(), 0, 100) . "...\n";
?>
```
