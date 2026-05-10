```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\PostProcessorConfig;

$config = new ExtractionConfig(
    postprocessor: new PostProcessorConfig(
        enabled: true,
        enabledProcessors: [
            'whitespace_normalizer',
            'unicode_normalizer'
        ]
    )
);

$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo "Processed content: " . substr($result->getContent(), 0, 100) . "...\n";
?>
```
