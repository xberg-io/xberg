```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\HtmlOutputConfig;

$config = new ExtractionConfig(
    resultFormat: 'html',
    htmlOutput: new HtmlOutputConfig(
        theme: 'github'
    )
);

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);

$result = $resultOutput->results[0];

// Output HTML with kb-* CSS classes
echo $result->getContent();
?>
```
