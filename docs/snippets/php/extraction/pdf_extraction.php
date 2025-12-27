```php
<?php

declare(strict_types=1);

/**
 * PDF Document Extraction
 *
 * Extract text, tables, and images from PDF files with various configurations.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\PdfConfig;

$kreuzberg = new Kreuzberg();
$result = $kreuzberg->extractFile('document.pdf');

echo "PDF Extraction Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Content length: " . strlen($result->content) . " characters\n";
echo "Tables found: " . count($result->tables) . "\n";
echo "Pages: " . ($result->metadata->pageCount ?? 'unknown') . "\n\n";

$config = new ExtractionConfig(
    extractImages: true,
    extractTables: true,
    pdf: new PdfConfig(
        extractImages: true,
        imageQuality: 85
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('report.pdf');

echo "Extracted Tables:\n";
echo str_repeat('=', 60) . "\n";
foreach ($result->tables as $index => $table) {
    echo "Table " . ($index + 1) . " (Page {$table->pageNumber}):\n";
    echo "Rows: " . count($table->cells) . "\n";
    echo "Columns: " . (count($table->cells[0] ?? []) ?? 0) . "\n\n";

    echo "Markdown format:\n";
    echo $table->markdown . "\n\n";

    $csvFile = "table_{$index}.csv";
    $fp = fopen($csvFile, 'w');
    foreach ($table->cells as $row) {
        fputcsv($fp, $row);
    }
    fclose($fp);
    echo "Saved to: $csvFile\n\n";
}

echo "Extracted Images:\n";
echo str_repeat('=', 60) . "\n";
foreach ($result->images ?? [] as $image) {
    $filename = sprintf(
        'page_%d_image_%d.%s',
        $image->pageNumber,
        $image->imageIndex,
        $image->format
    );

    file_put_contents($filename, $image->data);
    echo "Saved: $filename\n";
    echo "  Size: {$image->width}x{$image->height}\n";
    echo "  Format: {$image->format}\n";
    echo "  Data size: " . strlen($image->data) . " bytes\n\n";
}

$formattedConfig = new ExtractionConfig(
    preserveFormatting: true,
    outputFormat: 'markdown'
);

$kreuzberg = new Kreuzberg($formattedConfig);
$result = $kreuzberg->extractFile('formatted.pdf');

file_put_contents('output.md', $result->content);
echo "Saved formatted output to: output.md\n";

$result = $kreuzberg->extractFile('document.pdf');
$content = $result->content;

$sections = [];
$lines = explode("\n", $content);
$currentSection = null;
$currentContent = [];

foreach ($lines as $line) {
    if (preg_match('/^#+\s+(.+)$/', $line, $matches)) {
        if ($currentSection !== null) {
            $sections[$currentSection] = implode("\n", $currentContent);
        }
        $currentSection = $matches[1];
        $currentContent = [];
    } else {
        $currentContent[] = $line;
    }
}

if ($currentSection !== null) {
    $sections[$currentSection] = implode("\n", $currentContent);
}

echo "\nDocument sections:\n";
foreach ($sections as $title => $content) {
    echo "  - $title (" . strlen($content) . " chars)\n";
}
```
