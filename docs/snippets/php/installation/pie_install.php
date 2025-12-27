```php
<?php

declare(strict_types=1);

/**
 * Installing Kreuzberg PHP Extension using PIE
 *
 * PIE (PHP Installer for Extensions) is a modern tool for installing PHP extensions.
 * This snippet shows how to install the Kreuzberg extension using PIE.
 */









require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;

echo "Kreuzberg Extension Installation Check\n";
echo "========================================\n\n";

if (extension_loaded('kreuzberg')) {
    echo "✓ Kreuzberg extension is loaded\n";
    echo "  Version: " . Kreuzberg::version() . "\n\n";

    $info = [];
    ob_start();
    phpinfo(INFO_MODULES);
    $phpinfo = ob_get_clean();

    if (preg_match('/kreuzberg/i', $phpinfo)) {
        echo "✓ Extension info available via phpinfo()\n\n";
    }

    try {
        $kreuzberg = new Kreuzberg();
        echo "✓ Kreuzberg client initialized successfully\n\n";

        echo "Installation complete!\n";
        echo "You can now use Kreuzberg in your PHP applications.\n";
    } catch (Exception $e) {
        echo "✗ Error initializing Kreuzberg: {$e->getMessage()}\n";
    }
} else {
    echo "✗ Kreuzberg extension is not loaded\n\n";

    echo "Troubleshooting:\n";
    echo "================\n";
    echo "1. Make sure PIE installation completed successfully\n";
    echo "2. Check that extension is enabled in php.ini\n";
    echo "3. Restart your web server/PHP-FPM\n";
    echo "4. Run: php -m | grep kreuzberg\n";
    echo "5. Check error logs for loading issues\n\n";

    echo "Manual Installation:\n";
    echo "===================\n";
    echo "If PIE installation fails, try manual installation:\n";
    echo "1. Download extension from GitHub releases\n";
    echo "2. Copy .so/.dll file to PHP extension directory\n";
    echo "3. Add 'extension=kreuzberg.so' to php.ini\n";
    echo "4. Restart PHP\n";
}

echo "\n\nPIE Commands Reference:\n";
echo "=======================\n";
echo "Install extension:        pie install kreuzberg/kreuzberg-ext\n";
echo "Install specific version: pie install kreuzberg/kreuzberg-ext:4.0.0\n";
echo "List installed:           pie list\n";
echo "Update extension:         pie update kreuzberg/kreuzberg-ext\n";
echo "Uninstall:                pie uninstall kreuzberg/kreuzberg-ext\n";
echo "Show info:                pie info kreuzberg/kreuzberg-ext\n";

echo "\n\nNext Steps:\n";
echo "===========\n";
echo "1. Install Composer package: composer require kreuzberg/kreuzberg\n";
echo "2. Install optional dependencies:\n";
echo "   - Tesseract OCR: brew install tesseract (macOS) or apt install tesseract-ocr (Linux)\n";
echo "   - ONNX Runtime: brew install onnxruntime (macOS) or apt install libonnxruntime (Linux)\n";
echo "3. Start extracting documents!\n";
```
