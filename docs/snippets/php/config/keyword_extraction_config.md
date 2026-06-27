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
        'minScore' => 0.1,
        'language' => 'en',
    ],
], JSON_THROW_ON_ERROR));

$resultOutput = Xberg::extract(ExtractInput::fromUri('document.pdf'), $config);

$result = $resultOutput->results[0];

if ($result->extractedKeywords) {
    foreach ($result->extractedKeywords as $keyword) {
        echo $keyword->text . "\n";
    }
}
?>
```
