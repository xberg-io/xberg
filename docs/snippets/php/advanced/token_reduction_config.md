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

$result = Xberg::extractSync('document.pdf', null, $config);

echo "Reduced content: " . substr($result->getContent(), 0, 100) . "...\n";
?>
```
