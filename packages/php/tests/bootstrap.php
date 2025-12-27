<?php

declare(strict_types=1);

/**
 * PHPUnit bootstrap file for Kreuzberg PHP tests.
 *
 * This file is loaded before any tests run. It sets up the test environment,
 * loads the Composer autoloader, and checks for the Kreuzberg extension.
 */
$autoloader = dirname(__DIR__) . '/vendor/autoload.php';

if (!file_exists($autoloader)) {
    fwrite(
        STDERR,
        'Composer autoloader not found. Please run: composer install' . PHP_EOL,
    );
    exit(1);
}

require_once $autoloader;

if (PHP_VERSION_ID < 80200) {
    fwrite(
        STDERR,
        sprintf(
            'Kreuzberg requires PHP 8.2 or higher. Current version: %s' . PHP_EOL,
            PHP_VERSION,
        ),
    );
    exit(1);
}

if (extension_loaded('kreuzberg')) {
    fwrite(
        STDOUT,
        sprintf(
            '✓ Kreuzberg extension loaded (version %s)' . PHP_EOL,
            phpversion('kreuzberg') ?: 'unknown',
        ),
    );
} else {
    fwrite(
        STDOUT,
        '⚠ Kreuzberg extension not loaded - some tests will be skipped' . PHP_EOL,
    );
}

error_reporting(E_ALL);
ini_set('display_errors', '1');

if (!ini_get('date.timezone')) {
    date_default_timezone_set('UTC');
}
