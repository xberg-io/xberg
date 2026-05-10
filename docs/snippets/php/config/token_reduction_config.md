```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\TokenReductionOptions;

$config = new ExtractionConfig(
    tokenReduction: new TokenReductionOptions(
        mode: 'moderate',
        preserveImportantWords: true
    )
);

$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

echo "Original token count: " . $result->getTokenCount() . "\n";
echo "Reduced content: " . substr($result->getContent(), 0, 100) . "...\n";
?>
```
