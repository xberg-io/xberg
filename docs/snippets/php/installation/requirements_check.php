```php
<?php

declare(strict_types=1);

/**
 * System Requirements Check
 *
 * Verify that your system meets all requirements for running Kreuzberg.
 */

echo "Kreuzberg System Requirements Check\n";
echo "====================================\n\n";

$requirements_met = true;

echo "PHP Version: " . PHP_VERSION;
if (version_compare(PHP_VERSION, '8.1.0', '>=')) {
    echo " ✓ (>= 8.1.0 required)\n";
} else {
    echo " ✗ (>= 8.1.0 required)\n";
    $requirements_met = false;
}

$required_extensions = ['json', 'mbstring'];
foreach ($required_extensions as $ext) {
    echo "Extension '$ext': ";
    if (extension_loaded($ext)) {
        echo "✓ Loaded\n";
    } else {
        echo "✗ Missing\n";
        $requirements_met = false;
    }
}

echo "Extension 'kreuzberg': ";
if (extension_loaded('kreuzberg')) {
    echo "✓ Loaded\n";
} else {
    echo "✗ Missing\n";
    $requirements_met = false;
}

$memory_limit = ini_get('memory_limit');
echo "\nMemory Limit: $memory_limit";
$memory_bytes = return_bytes($memory_limit);
if ($memory_bytes >= 128 * 1024 * 1024) {
    echo " ✓ (>= 128M recommended)\n";
} else {
    echo " ! (>= 128M recommended for large documents)\n";
}

echo "\n";
if ($requirements_met) {
    echo "✓ All requirements met! You're ready to use Kreuzberg.\n";
} else {
    echo "✗ Some requirements are not met. Please install missing components.\n";
    exit(1);
}

/**
 * Convert PHP memory limit notation to bytes
 */
function return_bytes(string $val): int
{
    $val = trim($val);
    $last = strtolower($val[strlen($val) - 1]);
    $val = (int) $val;

    switch ($last) {
        case 'g':
            $val *= 1024;
        case 'm':
            $val *= 1024;
        case 'k':
            $val *= 1024;
    }

    return $val;
}
```
