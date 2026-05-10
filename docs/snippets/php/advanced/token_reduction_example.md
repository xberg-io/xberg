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

$result = Kreuzberg::extractFileSync('verbose_document.pdf', null, $config);

if ($result->getTokenCount() !== null) {
    echo "Original token count: " . $result->getTokenCount() . "\n";
}

// Access the reduced content
echo "Reduced content length: " . strlen($result->getContent()) . " characters\n";
echo "Content preview: " . substr($result->getContent(), 0, 100) . "...\n";
?>
```
