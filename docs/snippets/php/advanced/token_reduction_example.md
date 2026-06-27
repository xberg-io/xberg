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

$result = Xberg::extractSync('verbose_document.pdf', null, $config);

if ($result->getTokenCount() !== null) {
    echo "Original token count: " . $result->getTokenCount() . "\n";
}

// Access the reduced content
echo "Reduced content length: " . strlen($result->getContent()) . " characters\n";
echo "Content preview: " . substr($result->getContent(), 0, 100) . "...\n";
?>
```
