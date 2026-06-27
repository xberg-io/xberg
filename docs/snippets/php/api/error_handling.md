```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\XbergException;

$config = new ExtractionConfig();
try {
    $result = Xberg::extractSync('document.pdf', null, $config);
    echo $result->getContent();
} catch (XbergException $e) {
    // The extension throws XbergException with the error message
    // Error context is available in the exception message
    echo "Extraction failed: " . $e->getMessage() . "\n";
    echo "Error code: " . $e->getCode() . "\n";
}
```
