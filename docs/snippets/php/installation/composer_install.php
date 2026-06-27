```php title="composer_install.php"
<?php

declare(strict_types=1);

/**
 * Installing Xberg via Composer
 *
 * This snippet shows how to install the Xberg PHP package using Composer.
 * The package provides the object-oriented and procedural APIs, while the
 * native extension (xberg.so/.dll) must be installed separately.
 */


require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;

if (!extension_loaded('xberg')) {
    echo "Error: xberg extension is not loaded\n";
    echo "Please add 'extension=xberg.so' (or .dll on Windows) to your php.ini\n";
    exit(1);
}

echo "Xberg extension is loaded successfully!\n";
echo "Version: " . Xberg::version() . "\n";

echo "Xberg client initialized successfully!\n";
```
