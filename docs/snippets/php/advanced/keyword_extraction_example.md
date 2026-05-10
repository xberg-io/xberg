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
        minScore: 0.3,
        language: 'en'
    )
);

$result = Kreuzberg::extractFileSync('research_paper.pdf', null, $config);

if ($result->getKeywords()) {
    echo "Extracted Keywords:\n";
    foreach ($result->getKeywords() as $index => $keyword) {
        echo ($index + 1) . ". " . $keyword . "\n";
    }
} else {
    echo "No keywords extracted.\n";
}
?>
```
