```php
<?php

declare(strict_types=1);

/**
 * Excel Spreadsheet Extraction
 *
 * This example demonstrates extracting content from Excel files (.xlsx, .xls).
 * Excel spreadsheets are automatically converted to tables and text.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;

echo "Example 1: Basic Excel Extraction\n";
echo "=================================\n";

$kreuzberg = new Kreuzberg();
$result = $kreuzberg->extractFile('financial_report.xlsx');

echo "Content:\n";
echo $result->content . "\n\n";

echo "Metadata:\n";
echo "- Title: " . ($result->metadata->title ?? 'N/A') . "\n";
echo "- Author: " . (isset($result->metadata->authors) ? implode(', ', $result->metadata->authors) : 'N/A') . "\n";
echo "- Created: " . ($result->metadata->createdAt ?? 'N/A') . "\n";
echo "- Modified: " . ($result->metadata->modifiedAt ?? 'N/A') . "\n\n";

echo "Example 2: Extract Excel Tables\n";
echo "===============================\n";

$config2 = new ExtractionConfig(
    extractTables: true  
);

$result2 = (new Kreuzberg($config2))->extractFile('data.xlsx');

if (count($result2->tables) > 0) {
    echo "Found " . count($result2->tables) . " table(s)\n\n";

    foreach ($result2->tables as $i => $table) {
        echo "Table " . ($i + 1) . " (Sheet/Page {$table->pageNumber}):\n";
        echo $table->markdown . "\n\n";

        echo "Raw data:\n";
        echo "Rows: " . count($table->cells) . "\n";
        echo "Columns: " . (count($table->cells) > 0 ? count($table->cells[0]) : 0) . "\n\n";
    }
}

echo "Example 3: Convert Excel to CSV\n";
echo "===============================\n";

$result3 = $kreuzberg->extractFile('spreadsheet.xlsx');

foreach ($result3->tables as $i => $table) {
    $csvFilename = "sheet_{$i}.csv";
    $fp = fopen($csvFilename, 'w');

    foreach ($table->cells as $row) {
        fputcsv($fp, $row);
    }

    fclose($fp);
    echo "Saved: {$csvFilename}\n";
}

echo "\n";

echo "Example 4: Convert Excel to JSON\n";
echo "================================\n";

$result4 = $kreuzberg->extractFile('data.xlsx');

foreach ($result4->tables as $i => $table) {
    $jsonData = [];

    if (count($table->cells) > 0) {
        $headers = $table->cells[0];

        for ($j = 1; $j < count($table->cells); $j++) {
            $row = $table->cells[$j];
            $rowData = [];

            for ($k = 0; $k < count($headers); $k++) {
                $header = $headers[$k];
                $value = $row[$k] ?? '';
                $rowData[$header] = $value;
            }

            $jsonData[] = $rowData;
        }
    }

    $jsonFilename = "sheet_{$i}.json";
    file_put_contents($jsonFilename, json_encode($jsonData, JSON_PRETTY_PRINT));
    echo "Saved: {$jsonFilename}\n";
}

echo "\n";

echo "Example 5: Process Multiple Sheets\n";
echo "==================================\n";

$result5 = $kreuzberg->extractFile('multi_sheet_workbook.xlsx');

echo "Total sheets/tables: " . count($result5->tables) . "\n\n";

foreach ($result5->tables as $i => $table) {
    echo "Sheet " . ($i + 1) . ":\n";
    echo "- Rows: " . count($table->cells) . "\n";
    echo "- Columns: " . (count($table->cells) > 0 ? count($table->cells[0]) : 0) . "\n";

    if (count($table->cells) > 1) {  
        $numericColumns = [];

        for ($col = 0; $col < count($table->cells[0]); $col++) {
            $isNumeric = true;

            for ($row = 1; $row < count($table->cells); $row++) {
                $value = $table->cells[$row][$col] ?? '';
                if (!is_numeric(trim($value)) && trim($value) !== '') {
                    $isNumeric = false;
                    break;
                }
            }

            if ($isNumeric) {
                $numericColumns[] = $col;
            }
        }

        if (!empty($numericColumns)) {
            echo "- Numeric columns: " . count($numericColumns) . "\n";

            $col = $numericColumns[0];
            $sum = 0;
            for ($row = 1; $row < count($table->cells); $row++) {
                $value = $table->cells[$row][$col] ?? '0';
                $sum += (float) $value;
            }

            $columnName = $table->cells[0][$col] ?? "Column {$col}";
            echo "- Sum of '{$columnName}': {$sum}\n";
        }
    }

    echo "\n";
}

echo "Example 6: Extract Specific Data\n";
echo "================================\n";

$result6 = $kreuzberg->extractFile('budget.xlsx');

if (count($result6->tables) > 0) {
    $table = $result6->tables[0];

    echo "Header row:\n";
    if (count($table->cells) > 0) {
        print_r($table->cells[0]);
    }

    echo "\nFirst data row:\n";
    if (count($table->cells) > 1) {
        print_r($table->cells[1]);
    }

    if (count($table->cells) > 1 && count($table->cells[1]) > 2) {
        $cellValue = $table->cells[1][2];  
        echo "\nCell [1][2]: {$cellValue}\n";
    }
}

echo "\n";

echo "Example 7: Batch Process Excel Files\n";
echo "====================================\n";

$excelFiles = [
    'january_sales.xlsx',
    'february_sales.xlsx',
    'march_sales.xlsx',
];

$results = $kreuzberg->batchExtractFiles($excelFiles);

$totalSheets = 0;
foreach ($results as $i => $result) {
    $sheetCount = count($result->tables);
    $totalSheets += $sheetCount;

    echo "{$excelFiles[$i]}:\n";
    echo "- Sheets: {$sheetCount}\n";
    echo "- Text length: " . strlen($result->content) . " characters\n\n";
}

echo "Total sheets across all files: {$totalSheets}\n\n";

echo "Example 8: Convert Excel to HTML\n";
echo "================================\n";

$result8 = $kreuzberg->extractFile('report.xlsx');

foreach ($result8->tables as $i => $table) {
    $html = "<table border='1'>\n";

    foreach ($table->cells as $rowIndex => $row) {
        $html .= "  <tr>\n";

        $tag = $rowIndex === 0 ? 'th' : 'td';  

        foreach ($row as $cell) {
            $escapedCell = htmlspecialchars($cell);
            $html .= "    <{$tag}>{$escapedCell}</{$tag}>\n";
        }

        $html .= "  </tr>\n";
    }

    $html .= "</table>\n";

    $htmlFilename = "sheet_{$i}.html";
    file_put_contents($htmlFilename, $html);
    echo "Saved: {$htmlFilename}\n";
}

echo "\n";

echo "Example 9: Excel Metadata Extraction\n";
echo "====================================\n";

$result9 = $kreuzberg->extractFile('workbook.xlsx');

echo "File Metadata:\n";
echo "- Title: " . ($result9->metadata->title ?? 'N/A') . "\n";
echo "- Subject: " . ($result9->metadata->subject ?? 'N/A') . "\n";
echo "- Authors: " . (isset($result9->metadata->authors) ? implode(', ', $result9->metadata->authors) : 'N/A') . "\n";
echo "- Created: " . ($result9->metadata->createdAt ?? 'N/A') . "\n";
echo "- Modified: " . ($result9->metadata->modifiedAt ?? 'N/A') . "\n";
echo "- Created By: " . ($result9->metadata->createdBy ?? 'N/A') . "\n";
echo "- Keywords: " . (isset($result9->metadata->keywords) ? implode(', ', $result9->metadata->keywords) : 'N/A') . "\n";

if (!empty($result9->metadata->custom)) {
    echo "\nCustom Properties:\n";
    foreach ($result9->metadata->custom as $key => $value) {
        echo "- {$key}: {$value}\n";
    }
}

echo "\n";

echo "Example 10: Error Handling\n";
echo "=========================\n";

use Kreuzberg\Exceptions\KreuzbergException;

try {
    $result = $kreuzberg->extractFile('protected.xlsx');
    echo "Success: Extracted " . count($result->tables) . " sheets\n";
} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n";
    echo "Note: Password-protected files may require special handling\n";
}

echo "\n\nSupported Excel Formats:\n";
echo "========================\n";
echo "- .xlsx (Office Open XML)\n";
echo "- .xls (Legacy Excel format)\n";
echo "- .xlsm (Macro-enabled)\n";
echo "- .xlsb (Binary workbook)\n";
echo "- .xltx (Template)\n";

echo "\n\nBest Practices:\n";
echo "===============\n";
echo "1. Excel tables are automatically detected as Table objects\n";
echo "2. Each sheet becomes a separate table\n";
echo "3. Use table->cells for programmatic access to cell data\n";
echo "4. Use table->markdown for human-readable output\n";
echo "5. First row is often headers - handle accordingly\n";
echo "6. Check for numeric columns to perform calculations\n";
echo "7. Export to CSV/JSON for database import\n";
echo "8. Use batch processing for multiple Excel files\n";
```
