```php
<?php

declare(strict_types=1);

/**
 * Basic CLI Usage
 *
 * Simple command-line interface for document extraction.
 * Process documents from the terminal.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use function Kreuzberg\extract_file;

$options = getopt('f:o:h', ['file:', 'output:', 'help']);

if (isset($options['h']) || isset($options['help']) || empty($argv[1])) {
    echo "Kreuzberg Document Extraction CLI\n";
    echo str_repeat('=', 60) . "\n\n";
    echo "Usage: php basic_cli.php [options]\n\n";
    echo "Options:\n";
    echo "  -f, --file <path>      Input file to extract\n";
    echo "  -o, --output <path>    Output file (default: stdout)\n";
    echo "  -h, --help             Show this help message\n\n";
    echo "Examples:\n";
    echo "  php basic_cli.php -f document.pdf\n";
    echo "  php basic_cli.php -f report.docx -o output.txt\n";
    exit(0);
}

$inputFile = $options['f'] ?? $options['file'] ?? $argv[1] ?? null;

if ($inputFile === null || !file_exists($inputFile)) {
    fwrite(STDERR, "Error: Input file not found: $inputFile\n");
    exit(1);
}

$outputFile = $options['o'] ?? $options['output'] ?? null;

try {
    fwrite(STDERR, "Extracting: $inputFile\n");
    $start = microtime(true);

    $result = extract_file($inputFile);

    $elapsed = microtime(true) - $start;
    fwrite(STDERR, "Extraction completed in " . number_format($elapsed, 3) . "s\n");
    fwrite(STDERR, "Content length: " . strlen($result->content) . " characters\n");
    fwrite(STDERR, "Tables found: " . count($result->tables) . "\n");

    if ($outputFile) {
        file_put_contents($outputFile, $result->content);
        fwrite(STDERR, "Saved to: $outputFile\n");
    } else {
        echo $result->content;
    }

    exit(0);
} catch (\Kreuzberg\Exceptions\KreuzbergException $e) {
    fwrite(STDERR, "Error: " . $e->getMessage() . "\n");
    exit(1);
}
```
