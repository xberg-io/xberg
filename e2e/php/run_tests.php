<?php
// E2E test runner that configures the PHP extension before running tests
// Copies the built extension to a temp location and configures it in php.ini

$ext_src = realpath(__DIR__ . '/../../target/release');
if (!is_dir($ext_src)) {
    $ext_src = realpath(__DIR__ . '/../../target/debug');
}

// Determine platform-specific extension filename
$os = strtolower(php_uname('s'));
if (strpos($os, 'darwin') !== false || strpos($os, 'macos') !== false) {
    $ext_file = 'libkreuzberg_php.dylib';
} elseif (strpos($os, 'windows') !== false) {
    $ext_file = 'kreuzberg_php.dll';
} else {
    $ext_file = 'libkreuzberg_php.so';
}

$ext_path = $ext_src . '/' . $ext_file;

if (!file_exists($ext_path)) {
    fprintf(STDERR, "Error: Extension not found at %s\n", $ext_path);
    exit(1);
}

// Get PHP extension directory
$ext_dir = ini_get('extension_dir');

// Copy extension to temp location in extension_dir
// (needed because dl() is disabled, so PHP reads from extension_dir at startup)
$temp_ext = $ext_dir . '/' . 'kreuzberg_php_' . bin2hex(random_bytes(4)) . '_' . $ext_file;
if (!@copy($ext_path, $temp_ext)) {
    fprintf(STDERR, "Error: Could not copy extension to %s\n", $temp_ext);
    exit(1);
}
register_shutdown_function(function() use ($temp_ext) {
    @unlink($temp_ext);
});

// Create a temporary php.ini that loads the extension
$temp_ini = tempnam(sys_get_temp_dir(), 'phpkreuz');
$ini_content = file_get_contents(__DIR__ . '/php.ini');
$ini_content = preg_replace('/extension=kreuzberg_php/', 'extension=' . basename($temp_ext), $ini_content);
file_put_contents($temp_ini, $ini_content);

// Run PHPUnit with the temporary php.ini
$phpunit = __DIR__ . '/vendor/bin/phpunit';
$cmd = sprintf('php -c %s %s', escapeshellarg($temp_ini), escapeshellarg($phpunit));
passthru($cmd, $ret);
exit($ret);
