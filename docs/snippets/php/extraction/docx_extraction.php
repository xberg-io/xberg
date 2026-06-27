```php title="docx_extraction.php"
<?php

declare(strict_types=1);

/**
 * DOCX (Word) Document Extraction
 *
 * Extract text, tables, and metadata from Microsoft Word documents.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('document.docx'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Word Document Extraction:\n";
echo str_repeat('=', 60) . "\n";
echo "Content:\n";
echo $result->getContent() . "\n\n";

echo "Document Metadata:\n";
echo str_repeat('=', 60) . "\n";
echo "Title: " . ($result->metadata?->title ?? 'N/A') . "\n";
echo "Authors: " . (isset($result->metadata?->authors) ? implode(', ', $result->metadata?->authors) : 'N/A') . "\n";
echo "Created: " . ($result->metadata->createdAt ?? 'N/A') . "\n";
echo "Modified: " . ($result->metadata->modifiedAt ?? 'N/A') . "\n";
echo "Subject: " . ($result->metadata->subject ?? 'N/A') . "\n";
echo "Keywords: " . implode(', ', $result->metadata->keywords ?? []) . "\n\n";

$config = new ExtractionConfig(
    extractTables: true,
    preserveFormatting: true
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('report.docx'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

foreach ($result->tables as $index => $table) {
    echo "Table " . ($index + 1) . ":\n";
    echo str_repeat('-', 60) . "\n";

    foreach ($table->cells as $rowIndex => $row) {
        echo implode(' | ', $row) . "\n";
        if ($rowIndex === 0) {
            echo str_repeat('-', 60) . "\n";
        }
    }
    echo "\n";
}

$conversions = [
    'plain' => null,
    'markdown' => 'markdown',
];

foreach ($conversions as $name => $format) {
    $config = new ExtractionConfig(
        outputFormat: $format,
        preserveFormatting: $format !== null
    );

    $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('document.docx'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

    $outputFile = "output_$name.txt";
    file_put_contents($outputFile, $result->getContent());
    echo "Saved $name format to: $outputFile\n";
}

$docxFiles = glob('*.docx');
if (!empty($docxFiles)) {
    echo "\nBatch processing " . count($docxFiles) . " DOCX files...\n";

    $inputs = array_map(
        static fn (string $file): \Xberg\ExtractInput => \Xberg\ExtractInput::uri($file),
        $docxFiles
    );
    $output = Xberg::extractBatch($inputs, \Xberg\ExtractionConfig::default());
    $results = $output->results;

    foreach ($results as $index => $result) {
        $filename = basename($docxFiles[$index]);
        echo "\n$filename:\n";
        echo "  Characters: " . strlen($result->getContent()) . "\n";
        echo "  Tables: " . count($result->tables) . "\n";
        echo "  Authors: " . (isset($result->metadata?->authors) ? implode(', ', $result->metadata?->authors) : 'Unknown') . "\n";
    }
}

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('reviewed_document.docx'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

if (!empty($result->metadata->createdBy)) {
    echo "\nDocument Information:\n";
    echo "Created by: " . $result->metadata->createdBy . "\n";
}

if (!empty($result->metadata->producer)) {
    echo "Producer: " . $result->metadata->producer . "\n";
}

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('document.docx'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];
$content = $result->getContent();

$stats = [
    'characters' => mb_strlen($content),
    'words' => str_word_count($content),
    'lines' => substr_count($content, "\n"),
    'paragraphs' => substr_count($content, "\n\n"),
    'sentences' => preg_match_all('/[.!?]+/', $content),
];

echo "\nDocument Statistics:\n";
echo str_repeat('=', 60) . "\n";
foreach ($stats as $metric => $value) {
    echo ucfirst($metric) . ": " . number_format($value) . "\n";
}
```
