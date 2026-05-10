```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\KreuzbergException;

$config = new ExtractionConfig();
try {
    $result = Kreuzberg::extractFileSync('document.pdf', null, $config);
    echo $result->getContent();
} catch (KreuzbergException $e) {
    // The extension throws KreuzbergException with the error message
    // Error context is available in the exception message
    echo "Extraction failed: " . $e->getMessage() . "\n";
    echo "Error code: " . $e->getCode() . "\n";
}
```
