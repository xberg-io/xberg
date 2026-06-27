```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\XbergException;

$config = ExtractionConfig::default();
try {
    $resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);
    $result = $resultOutput->results[0];
    echo $result->getContent();
} catch (XbergException $e) {
    // The extension throws XbergException with the error message
    // Error context is available in the exception message
    echo "Extraction failed: " . $e->getMessage() . "\n";
    echo "Error code: " . $e->getCode() . "\n";
}
```
