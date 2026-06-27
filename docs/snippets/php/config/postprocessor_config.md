```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\PostProcessorConfig;

$config = new ExtractionConfig(
    postprocessor: new PostProcessorConfig(
        enabled: true,
        enabledProcessors: [
            'whitespace_normalizer',
            'unicode_normalizer'
        ]
    )
);

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);

$result = $resultOutput->results[0];

echo "Processed content: " . substr($result->getContent(), 0, 100) . "...\n";
?>
```
