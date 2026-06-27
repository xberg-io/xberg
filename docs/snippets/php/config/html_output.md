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

$result = Xberg::extractSync('document.pdf', null, $config);

// Output HTML with kb-* CSS classes
echo $result->getContent();
?>
```
