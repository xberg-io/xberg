```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\KeywordConfig;

$config = new ExtractionConfig(
    keywords: new KeywordConfig(
        algorithm: 'yake',
        maxKeywords: 10,
        minScore: 0.1,
        language: 'en'
    )
);

$result = Xberg::extractSync('document.pdf', null, $config);

if ($result->getKeywords()) {
    foreach ($result->getKeywords() as $keyword) {
        echo $keyword . "\n";
    }
}
?>
```
