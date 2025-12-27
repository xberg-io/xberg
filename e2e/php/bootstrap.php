<?php

declare(strict_types=1);

/**
 * PHPUnit bootstrap file for E2E tests.
 *
 * This file is loaded before running the test suite and performs the following:
 * - Loads the Composer autoloader for test dependencies
 * - Verifies the Kreuzberg PHP extension is loaded
 * - Sets up the test environment
 */

$autoloadPaths = [
    __DIR__ . '/../../packages/php/vendor/autoload.php',
    __DIR__ . '/vendor/autoload.php',
];

$autoloaded = false;
foreach ($autoloadPaths as $path) {
    if (file_exists($path)) {
        require_once $path;
        $autoloaded = true;
        break;
    }
}

if (!$autoloaded) {
    fwrite(
        STDERR,
        "Error: Could not find Composer autoloader.\n" .
        "Please run 'composer install' in the packages/php directory.\n"
    );
    exit(1);
}

if (!extension_loaded('kreuzberg')) {
    fwrite(
        STDERR,
        "Error: Kreuzberg PHP extension is not loaded.\n" .
        "Please build and install the extension first, then run with:\n" .
        "  php -dextension=packages/php/ext/libkreuzberg.dylib vendor/bin/phpunit\n"
    );
    exit(1);
}

$workspaceRoot = realpath(__DIR__ . '/../..');
$testDocuments = $workspaceRoot . '/test_documents';

if (!is_dir($testDocuments)) {
    fwrite(
        STDERR,
        "Error: test_documents directory not found at: {$testDocuments}\n" .
        "Please ensure the test_documents directory exists in the workspace root.\n"
    );
    exit(1);
}

echo "PHPUnit E2E Test Suite Bootstrap\n";
echo "=================================\n";
echo "Workspace Root: {$workspaceRoot}\n";
echo "Test Documents: {$testDocuments}\n";
echo "Kreuzberg Extension: Loaded\n";
echo "\n";
