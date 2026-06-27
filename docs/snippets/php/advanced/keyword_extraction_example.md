```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractInput;
use Xberg\ExtractionConfig;

$config = ExtractionConfig::from_json(json_encode([
    'keywords' => [
        'algorithm' => 'yake',
        'maxKeywords' => 10,
        'minScore' => 0.3,
        'language' => 'en',
    ],
], JSON_THROW_ON_ERROR));

$resultOutput = Xberg::extract(ExtractInput::fromUri('research_paper.pdf'), $config);

$result = $resultOutput->results[0];

if ($result->extractedKeywords) {
    echo "Extracted Keywords:\n";
    foreach ($result->extractedKeywords as $index => $keyword) {
        echo ($index + 1) . ". " . $keyword->text . "\n";
    }
} else {
    echo "No keywords extracted.\n";
}
?>
```
