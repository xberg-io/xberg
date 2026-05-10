```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\HtmlOutputConfig;

$config = new ExtractionConfig(
    resultFormat: 'html',
    htmlOutput: new HtmlOutputConfig(
        theme: 'github'
    )
);

$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

// Output HTML with kb-* CSS classes
echo $result->getContent();
?>
```
