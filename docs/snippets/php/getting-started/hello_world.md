```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;

$result = Kreuzberg::extractFileSync('document.pdf', null, null);
echo "Hello, " . substr($result->getContent(), 0, 50) . "\n";
```
