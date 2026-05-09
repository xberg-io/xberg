```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;

// Clear all registered OCR backends
Kreuzberg::clearOcrBackends();

// Clear all registered post-processors
Kreuzberg::clearPostProcessors();

// Clear all registered validators
Kreuzberg::clearValidators();

echo "All plugins cleared\n";
```
