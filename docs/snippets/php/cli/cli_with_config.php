```php
<?php

declare(strict_types=1);

/**
 * Advanced CLI with Configuration
 *
 * Command-line tool with support for various extraction options.
 * Supports OCR, tables, images, and output formats.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\ChunkingConfig;

$longOpts = [
    'file:',
    'output:',
    'format:',
    'ocr',
    'ocr-lang:',
    'tables',
    'images',
    'chunks:',
    'help',
];

$options = getopt('f:o:', $longOpts);

if (isset($options['help']) || empty($options)) {
    echo "Kreuzberg Advanced CLI\n";
    echo str_repeat('=', 60) . "\n\n";
    echo "Usage: php cli_with_config.php [options]\n\n";
    echo "Options:\n";
    echo "  -f, --file <path>        Input file to extract (required)\n";
    echo "  -o, --output <path>      Output file (default: stdout)\n";
    echo "  --format <format>        Output format: text, json, markdown\n";
    echo "  --ocr                    Enable OCR for scanned documents\n";
    echo "  --ocr-lang <lang>        OCR language (default: eng)\n";
    echo "  --tables                 Extract tables\n";
    echo "  --images                 Extract images\n";
    echo "  --chunks <size>          Split into chunks of size\n";
    echo "  --help                   Show this help message\n\n";
    echo "Examples:\n";
    echo "  php cli_with_config.php --file scan.pdf --ocr\n";
    echo "  php cli_with_config.php --file report.pdf --tables --format json\n";
    echo "  php cli_with_config.php --file doc.pdf --chunks 512 --output chunks.json\n";
    exit(0);
}

$inputFile = $options['file'] ?? $options['f'] ?? null;

if ($inputFile === null || !file_exists($inputFile)) {
    fwrite(STDERR, "Error: Input file required and must exist\n");
    exit(1);
}

$enableOcr = isset($options['ocr']);
$ocrLang = $options['ocr-lang'] ?? 'eng';
$extractTables = isset($options['tables']);
$extractImages = isset($options['images']);
$chunkSize = isset($options['chunks']) ? (int)$options['chunks'] : null;
$format = $options['format'] ?? 'text';

$config = new ExtractionConfig(
    ocr: $enableOcr ? new OcrConfig(backend: 'tesseract', language: $ocrLang) : null,
    extractTables: $extractTables,
    extractImages: $extractImages,
    chunking: $chunkSize ? new ChunkingConfig(maxChunkSize: $chunkSize) : null,
    preserveFormatting: $format === 'markdown',
    outputFormat: $format === 'markdown' ? 'markdown' : null
);

try {
    fwrite(STDERR, "Processing: $inputFile\n");
    fwrite(STDERR, "Options:\n");
    fwrite(STDERR, "  OCR: " . ($enableOcr ? "enabled ($ocrLang)" : "disabled") . "\n");
    fwrite(STDERR, "  Tables: " . ($extractTables ? "enabled" : "disabled") . "\n");
    fwrite(STDERR, "  Images: " . ($extractImages ? "enabled" : "disabled") . "\n");
    fwrite(STDERR, "  Chunks: " . ($chunkSize ?? "disabled") . "\n");
    fwrite(STDERR, "  Format: $format\n\n");

    $start = microtime(true);
    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile($inputFile);
    $elapsed = microtime(true) - $start;

    fwrite(STDERR, "Extraction completed in " . number_format($elapsed, 3) . "s\n");

    $output = match ($format) {
        'json' => json_encode([
            'content' => $result->content,
            'metadata' => [
                'title' => $result->metadata->title,
                'author' => $result->metadata->author,
                'page_count' => $result->metadata->pageCount,
            ],
            'tables' => array_map(fn($t) => [
                'page' => $t->pageNumber,
                'markdown' => $t->markdown,
            ], $result->tables),
            'chunks' => $chunkSize ? array_map(fn($c) => [
                'index' => $c->metadata->chunkIndex,
                'content' => $c->content,
            ], $result->chunks ?? []) : null,
        ], JSON_PRETTY_PRINT),
        'markdown' => $result->content,
        default => $result->content,
    };

    $outputFile = $options['output'] ?? $options['o'] ?? null;
    if ($outputFile) {
        file_put_contents($outputFile, $output);
        fwrite(STDERR, "Output written to: $outputFile\n");
    } else {
        echo $output;
    }

    fwrite(STDERR, "\nStatistics:\n");
    fwrite(STDERR, "  Content: " . strlen($result->content) . " characters\n");
    fwrite(STDERR, "  Tables: " . count($result->tables) . "\n");
    fwrite(STDERR, "  Images: " . count($result->images ?? []) . "\n");
    fwrite(STDERR, "  Chunks: " . count($result->chunks ?? []) . "\n");

    exit(0);
} catch (\Kreuzberg\Exceptions\KreuzbergException $e) {
    fwrite(STDERR, "Error: " . $e->getMessage() . "\n");
    exit(1);
}
```
