```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\KeywordConfig;

$config = new ExtractionConfig(
    keywords: new KeywordConfig(
        algorithm: 'yake',
        maxKeywords: 10,
        minScore: 0.1,
        language: 'en'
    )
);

$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

if ($result->getKeywords()) {
    foreach ($result->getKeywords() as $keyword) {
        echo $keyword . "\n";
    }
}
?>
```
