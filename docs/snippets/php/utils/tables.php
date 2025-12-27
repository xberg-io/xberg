```php
<?php

declare(strict_types=1);

/**
 * Table Extraction and Processing
 *
 * Extract tables from documents and convert them to various formats.
 * Demonstrates table processing, formatting, and export capabilities.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Result\ExtractedTable;

$config = new ExtractionConfig(
    extractTables: true
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('document.pdf');

echo "Table Extraction Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Tables found: " . count($result->tables) . "\n\n";

foreach ($result->tables as $tableIndex => $table) {
    echo "Table " . ($tableIndex + 1) . ":\n";
    echo str_repeat('-', 40) . "\n";

    $rowCount = count($table->cells);
    $colCount = !empty($table->cells) ? count($table->cells[0]) : 0;

    echo "  Dimensions: $rowCount rows × $colCount columns\n";

    if (isset($table->pageNumber)) {
        echo "  Page: {$table->pageNumber}\n";
    }

    echo "\n";

    echo "  Markdown representation:\n";
    echo str_repeat('-', 40) . "\n";
    echo $table->markdown . "\n\n";

    echo "  Raw data preview:\n";
    echo str_repeat('-', 40) . "\n";

    $previewRows = array_slice($table->cells, 0, 3);
    foreach ($previewRows as $rowIndex => $row) {
        echo "  Row " . ($rowIndex + 1) . ": [" . implode(' | ', $row) . "]\n";
    }

    if ($rowCount > 3) {
        echo "  ... and " . ($rowCount - 3) . " more rows\n";
    }

    echo "\n";
}

echo "Exporting Tables to CSV:\n";
echo str_repeat('=', 60) . "\n";

$outputDir = './exported_tables';
if (!is_dir($outputDir)) {
    mkdir($outputDir, 0755, true);
}

foreach ($result->tables as $index => $table) {
    $filename = sprintf('table_%d.csv', $index + 1);
    $filepath = $outputDir . '/' . $filename;

    $fp = fopen($filepath, 'w');

    if ($fp !== false) {
        foreach ($table->cells as $row) {
            fputcsv($fp, $row);
        }

        fclose($fp);
        echo "Saved: $filename\n";
    } else {
        echo "Error: Failed to create $filename\n";
    }
}

echo "\n";

echo "Exporting Tables to JSON:\n";
echo str_repeat('=', 60) . "\n";

foreach ($result->tables as $index => $table) {
    $filename = sprintf('table_%d.json', $index + 1);
    $filepath = $outputDir . '/' . $filename;

    $tableData = [
        'index' => $index + 1,
        'page' => $table->pageNumber ?? null,
        'dimensions' => [
            'rows' => count($table->cells),
            'columns' => !empty($table->cells) ? count($table->cells[0]) : 0,
        ],
        'data' => $table->cells,
        'markdown' => $table->markdown,
    ];

    $json = json_encode($tableData, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE);
    file_put_contents($filepath, $json);

    echo "Saved: $filename\n";
}

echo "\n";

function tableToHtml(ExtractedTable $table): string
{
    $html = "<table>\n";

    foreach ($table->cells as $rowIndex => $row) {
        $html .= "  <tr>\n";

        $tag = $rowIndex === 0 ? 'th' : 'td';

        foreach ($row as $cell) {
            $escapedCell = htmlspecialchars($cell, ENT_QUOTES, 'UTF-8');
            $html .= "    <$tag>$escapedCell</$tag>\n";
        }

        $html .= "  </tr>\n";
    }

    $html .= "</table>";

    return $html;
}

echo "Exporting Tables to HTML:\n";
echo str_repeat('=', 60) . "\n";

foreach ($result->tables as $index => $table) {
    $filename = sprintf('table_%d.html', $index + 1);
    $filepath = $outputDir . '/' . $filename;

    $html = "<!DOCTYPE html>\n";
    $html .= "<html>\n<head>\n";
    $html .= "  <meta charset=\"UTF-8\">\n";
    $html .= "  <title>Table " . ($index + 1) . "</title>\n";
    $html .= "  <style>\n";
    $html .= "    table { border-collapse: collapse; width: 100%; }\n";
    $html .= "    th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }\n";
    $html .= "    th { background-color: #f2f2f2; }\n";
    $html .= "  </style>\n";
    $html .= "</head>\n<body>\n";
    $html .= "  <h1>Table " . ($index + 1) . "</h1>\n";
    $html .= tableToHtml($table) . "\n";
    $html .= "</body>\n</html>";

    file_put_contents($filepath, $html);

    echo "Saved: $filename\n";
}

echo "\n";

echo "Table Analysis:\n";
echo str_repeat('=', 60) . "\n";

foreach ($result->tables as $index => $table) {
    echo "Table " . ($index + 1) . " Analysis:\n";

    $cells = $table->cells;
    $totalCells = array_sum(array_map('count', $cells));
    $emptyCells = 0;
    $numericCells = 0;

    foreach ($cells as $row) {
        foreach ($row as $cell) {
            if (empty(trim($cell))) {
                $emptyCells++;
            }

            if (is_numeric($cell)) {
                $numericCells++;
            }
        }
    }

    echo "  Total cells: $totalCells\n";
    echo "  Empty cells: $emptyCells (" . number_format(($emptyCells / max($totalCells, 1)) * 100, 1) . "%)\n";
    echo "  Numeric cells: $numericCells (" . number_format(($numericCells / max($totalCells, 1)) * 100, 1) . "%)\n";

    $numericRatio = $numericCells / max($totalCells, 1);
    $tableType = match(true) {
        $numericRatio > 0.5 => 'Data/Numeric Table',
        $numericRatio > 0.2 => 'Mixed Content Table',
        default => 'Text Table',
    };

    echo "  Table type: $tableType\n\n";
}

function tableToAssociativeArray(ExtractedTable $table): array
{
    if (empty($table->cells)) {
        return [];
    }

    $headers = array_shift($table->cells);
    $data = [];

    foreach ($table->cells as $row) {
        $rowData = [];
        foreach ($headers as $index => $header) {
            $rowData[$header] = $row[$index] ?? '';
        }
        $data[] = $rowData;
    }

    return $data;
}

if (!empty($result->tables)) {
    $firstTable = $result->tables[0];
    $associativeData = tableToAssociativeArray($firstTable);

    echo "First Table as Associative Array:\n";
    echo str_repeat('=', 60) . "\n";
    echo json_encode(array_slice($associativeData, 0, 3), JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE) . "\n";

    if (count($associativeData) > 3) {
        echo "... and " . (count($associativeData) - 3) . " more records\n";
    }
}
```
