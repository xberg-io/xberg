```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;

// Unregister all OCR backends by clearing the registry
Kreuzberg::clearOcrBackends();

// Unregister all post-processors by clearing the registry
Kreuzberg::clearPostProcessors();

// Unregister all validators by clearing the registry
Kreuzberg::clearValidators();

echo "All plugins unregistered\n";
```
