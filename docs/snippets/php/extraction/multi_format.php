```php
<?php

declare(strict_types=1);

/**
 * Multi-Format Document Extraction
 *
 * Handle various document formats (PDF, DOCX, XLSX, PPTX, images, etc.)
 * with format-specific processing and unified output.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use function Kreuzberg\extract_file;
use function Kreuzberg\detect_mime_type_from_path;

$formats = [
    'PDF' => 'document.pdf',
    'Word' => 'document.docx',
    'Excel' => 'spreadsheet.xlsx',
    'PowerPoint' => 'presentation.pptx',
    'Text' => 'readme.txt',
    'HTML' => 'page.html',
    'Markdown' => 'guide.md',
    'Image' => 'scan.png',
];

echo "Multi-Format Extraction:\n";
echo str_repeat('=', 60) . "\n\n";

$kreuzberg = new Kreuzberg();

foreach ($formats as $type => $file) {
    if (!file_exists($file)) {
        continue;
    }

    echo "Processing $type ($file):\n";

    $mimeType = detect_mime_type_from_path($file);
    echo "  MIME type: $mimeType\n";

    $result = $kreuzberg->extractFile($file);

    echo "  Content length: " . strlen($result->content) . " chars\n";
    echo "  Tables: " . count($result->tables) . "\n";
    echo "  Images: " . count($result->images ?? []) . "\n";
    echo "  Pages: " . ($result->metadata->pageCount ?? 'N/A') . "\n";
    echo "\n";
}

$mixedFiles = glob('documents/*.*');
$byFormat = [];

foreach ($mixedFiles as $file) {
    $mimeType = detect_mime_type_from_path($file);
    $extension = pathinfo($file, PATHINFO_EXTENSION);

    if (!isset($byFormat[$extension])) {
        $byFormat[$extension] = [];
    }

    $result = extract_file($file);
    $byFormat[$extension][] = [
        'file' => basename($file),
        'mime' => $mimeType,
        'size' => strlen($result->content),
        'tables' => count($result->tables),
    ];
}

echo "Files by Format:\n";
echo str_repeat('=', 60) . "\n";
foreach ($byFormat as $ext => $files) {
    echo strtoupper($ext) . ": " . count($files) . " files\n";

    $totalSize = array_sum(array_column($files, 'size'));
    $totalTables = array_sum(array_column($files, 'tables'));

    echo "  Total content: " . number_format($totalSize) . " chars\n";
    echo "  Total tables: $totalTables\n\n";
}

$formatConfigs = [
    'pdf' => new ExtractionConfig(
        extractTables: true,
        extractImages: true,
        pdf: new \Kreuzberg\Config\PdfConfig(
            extractImages: true,
            imageQuality: 85
        )
    ),
    'docx' => new ExtractionConfig(
        extractTables: true,
        preserveFormatting: true
    ),
    'xlsx' => new ExtractionConfig(
        extractTables: true  
    ),
    'png' => new ExtractionConfig(
        ocr: new \Kreuzberg\Config\OcrConfig(
            backend: 'tesseract',
            language: 'eng'
        )
    ),
];

foreach ($mixedFiles as $file) {
    $ext = strtolower(pathinfo($file, PATHINFO_EXTENSION));

    if (!isset($formatConfigs[$ext])) {
        continue;
    }

    $config = $formatConfigs[$ext];
    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile($file);

    echo "Processed " . basename($file) . " with $ext config\n";
}

function convertToMarkdown(string $inputFile): string
{
    $config = new ExtractionConfig(
        preserveFormatting: true,
        outputFormat: 'markdown',
        extractTables: true
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile($inputFile);

    $markdown = "# " . ($result->metadata->title ?? basename($inputFile)) . "\n\n";

    if (isset($result->metadata->authors)) {
        $markdown .= "_Authors: " . implode(', ', $result->metadata->authors) . "_\n\n";
    }

    $markdown .= $result->content . "\n\n";

    foreach ($result->tables as $index => $table) {
        $markdown .= "## Table " . ($index + 1) . "\n\n";
        $markdown .= $table->markdown . "\n\n";
    }

    return $markdown;
}

echo "\nConverting to Markdown:\n";
echo str_repeat('=', 60) . "\n";

foreach (['document.pdf', 'document.docx'] as $file) {
    if (!file_exists($file)) {
        continue;
    }

    $markdown = convertToMarkdown($file);
    $outputFile = pathinfo($file, PATHINFO_FILENAME) . '.md';

    file_put_contents($outputFile, $markdown);
    echo "Converted: $file -> $outputFile\n";
}

function extractFromArchive(string $archiveFile): array
{
    $result = extract_file($archiveFile);

    return [
        'archive' => basename($archiveFile),
        'listing' => $result->content,
        'mime' => $result->mimeType,
    ];
}

class UniversalExtractor
{
    private Kreuzberg $kreuzberg;
    private array $formatHandlers = [];

    public function __construct()
    {
        $this->kreuzberg = new Kreuzberg();

        $this->formatHandlers = [
            'application/pdf' => [$this, 'handlePDF'],
            'application/vnd.openxmlformats-officedocument.wordprocessingml.document' => [$this, 'handleDOCX'],
            'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet' => [$this, 'handleXLSX'],
            'image/png' => [$this, 'handleImage'],
            'image/jpeg' => [$this, 'handleImage'],
        ];
    }

    public function extract(string $file): array
    {
        $mimeType = detect_mime_type_from_path($file);
        $handler = $this->formatHandlers[$mimeType] ?? [$this, 'handleGeneric'];

        return $handler($file, $mimeType);
    }

    private function handlePDF(string $file, string $mimeType): array
    {
        $config = new ExtractionConfig(extractTables: true, extractImages: true);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($file);

        return [
            'type' => 'PDF',
            'content' => $result->content,
            'tables' => count($result->tables),
            'images' => count($result->images ?? []),
            'pages' => $result->metadata->pageCount,
        ];
    }

    private function handleDOCX(string $file, string $mimeType): array
    {
        $result = $this->kreuzberg->extractFile($file);

        return [
            'type' => 'Word Document',
            'content' => $result->content,
            'tables' => count($result->tables),
            'authors' => $result->metadata->authors,
        ];
    }

    private function handleXLSX(string $file, string $mimeType): array
    {
        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($file);

        return [
            'type' => 'Excel Spreadsheet',
            'content' => $result->content,
            'sheets' => count($result->tables),  
        ];
    }

    private function handleImage(string $file, string $mimeType): array
    {
        $config = new ExtractionConfig(
            ocr: new \Kreuzberg\Config\OcrConfig(backend: 'tesseract', language: 'eng')
        );
        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($file);

        return [
            'type' => 'Image (OCR)',
            'content' => $result->content,
            'ocr_length' => strlen($result->content),
        ];
    }

    private function handleGeneric(string $file, string $mimeType): array
    {
        $result = $this->kreuzberg->extractFile($file);

        return [
            'type' => 'Generic',
            'mime' => $mimeType,
            'content' => $result->content,
        ];
    }
}

$extractor = new UniversalExtractor();

echo "\nUniversal Extraction:\n";
echo str_repeat('=', 60) . "\n";

foreach ($mixedFiles as $file) {
    $data = $extractor->extract($file);
    echo basename($file) . " ({$data['type']}):\n";
    print_r(array_filter($data, fn($k) => $k !== 'content', ARRAY_FILTER_USE_KEY));
    echo "\n";
}
```
