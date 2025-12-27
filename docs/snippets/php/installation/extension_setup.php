```php
<?php

declare(strict_types=1);

/**
 * Setting up the Kreuzberg PHP Extension
 *
 * The Kreuzberg native extension must be installed and loaded before using the library.
 * This snippet shows how to check for the extension and provides guidance for installation.
 */

if (!extension_loaded('kreuzberg')) {
    echo "Kreuzberg extension not found!\n\n";
    echo "Installation steps:\n";
    echo "1. Download the extension for your platform from:\n";
    echo "   https://github.com/kreuzberg-dev/kreuzberg/releases\n\n";
    echo "2. Copy the extension to your PHP extensions directory:\n";
    echo "   - Linux/macOS: kreuzberg.so\n";
    echo "   - Windows: kreuzberg.dll\n\n";
    echo "3. Add to your php.ini:\n";
    echo "   extension=kreuzberg.so  ; Linux/macOS\n";
    echo "   extension=kreuzberg.dll ; Windows\n\n";
    echo "4. Restart PHP/PHP-FPM/Apache\n\n";
    echo "5. Verify with: php -m | grep kreuzberg\n";
    exit(1);
}

echo "Kreuzberg Extension Information:\n";
echo "================================\n";
echo "Status: Loaded\n";

$tesseract_available = function_exists('kreuzberg_has_tesseract') ? kreuzberg_has_tesseract() : false;
$onnx_available = function_exists('kreuzberg_has_onnx') ? kreuzberg_has_onnx() : false;

echo "Tesseract OCR: " . ($tesseract_available ? "Available" : "Not available") . "\n";
echo "ONNX Runtime: " . ($onnx_available ? "Available" : "Not available") . "\n";

if (!$tesseract_available) {
    echo "\nTo enable OCR functionality, install Tesseract:\n";
    echo "  macOS: brew install tesseract\n";
    echo "  Ubuntu/Debian: sudo apt install tesseract-ocr\n";
}

if (!$onnx_available) {
    echo "\nTo enable embeddings, install ONNX Runtime:\n";
    echo "  macOS: brew install onnxruntime\n";
    echo "  Ubuntu/Debian: sudo apt install libonnxruntime\n";
}
```
