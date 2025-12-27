<?php

declare(strict_types=1);

/**
 * Example: Custom Extractor Plugin
 *
 * This example demonstrates how to implement and use custom extractors
 * to handle proprietary or custom document formats.
 */

require_once __DIR__ . '/../vendor/autoload.php';

use Kreuzberg\Plugins\ExtractorInterface;
use Kreuzberg\Plugins\ExtractorRegistry;


/**
 * Custom extractor for a hypothetical ".custom" format.
 *
 * This demonstrates the minimum required implementation of ExtractorInterface.
 */
class CustomFormatExtractor implements ExtractorInterface
{
    public function extract(string $bytes, string $mimeType): array
    {
        $lines = explode("\n", $bytes);
        $content = '';
        $metadata = [];

        foreach ($lines as $line) {
            if (str_starts_with($line, 'TITLE:')) {
                $metadata['title'] = trim(substr($line, 6));
            } elseif (str_starts_with($line, 'AUTHOR:')) {
                $metadata['author'] = trim(substr($line, 7));
            } else {
                $content .= $line . "\n";
            }
        }

        return [
            'content' => trim($content),
            'metadata' => $metadata,
            'tables' => [],
        ];
    }
}

ExtractorRegistry::register('text/x-custom', new CustomFormatExtractor());

$customDocument = <<<CUSTOM
TITLE: Sample Document
AUTHOR: John Doe

This is the content of the custom document.
It can span multiple lines.
CUSTOM;

try {
    $result = kreuzberg_extract_bytes($customDocument, 'text/x-custom');
    echo "=== Custom Format Extraction ===\n";
    echo "Content: {$result->content}\n";
    echo "Title: " . ($result->metadata['title'] ?? 'N/A') . "\n";
    echo "Author: " . ($result->metadata['author'] ?? 'N/A') . "\n";
    echo "\n";
} catch (Exception $e) {
    echo "Error: {$e->getMessage()}\n";
}


ExtractorRegistry::register('text/simple', function (string $bytes, string $mimeType): array {
    return [
        'content' => strtoupper($bytes),
        'metadata' => [
            'extractor' => 'closure-based',
            'length' => strlen($bytes),
        ],
        'tables' => [],
    ];
});

try {
    $result = kreuzberg_extract_bytes('hello world', 'text/simple');
    echo "=== Closure-based Extraction ===\n";
    echo "Content: {$result->content}\n";
    echo "Length: " . $result->metadata['length'] . "\n";
    echo "\n";
} catch (Exception $e) {
    echo "Error: {$e->getMessage()}\n";
}


class JsonExtractor implements ExtractorInterface
{
    public function extract(string $bytes, string $mimeType): array
    {
        $data = json_decode($bytes, true);

        if (json_last_error() !== JSON_ERROR_NONE) {
            throw new RuntimeException('Invalid JSON: ' . json_last_error_msg());
        }

        $content = $this->extractStrings($data);

        $tables = [];
        if (isset($data['tables']) && is_array($data['tables'])) {
            foreach ($data['tables'] as $tableData) {
                $tables[] = [
                    'cells' => $tableData['cells'] ?? [],
                    'markdown' => $this->convertToMarkdown($tableData['cells'] ?? []),
                    'page_number' => $tableData['page'] ?? 1,
                ];
            }
        }

        return [
            'content' => $content,
            'metadata' => [
                'extractor' => 'json',
                'fields_count' => $this->countFields($data),
            ],
            'tables' => $tables,
        ];
    }

    private function extractStrings(mixed $value): string
    {
        if (is_string($value)) {
            return $value . "\n";
        }

        if (is_array($value)) {
            $result = '';
            foreach ($value as $item) {
                $result .= $this->extractStrings($item);
            }
            return $result;
        }

        return '';
    }

    private function countFields(array $data): int
    {
        $count = 0;
        foreach ($data as $key => $value) {
            if (is_array($value)) {
                $count += $this->countFields($value);
            } else {
                $count++;
            }
        }
        return $count;
    }

    private function convertToMarkdown(array $cells): string
    {
        if (empty($cells)) {
            return '';
        }

        $markdown = '';
        foreach ($cells as $rowIndex => $row) {
            $markdown .= '| ' . implode(' | ', $row) . ' |' . "\n";

            if ($rowIndex === 0) {
                $markdown .= '|' . str_repeat(' --- |', count($row)) . "\n";
            }
        }

        return $markdown;
    }
}

ExtractorRegistry::register('application/json', new JsonExtractor());

$jsonData = json_encode([
    'title' => 'Document Title',
    'content' => 'Main content text',
    'tables' => [
        [
            'cells' => [
                ['Name', 'Age'],
                ['John', '30'],
                ['Jane', '25'],
            ],
            'page' => 1,
        ],
    ],
]);

try {
    $result = kreuzberg_extract_bytes($jsonData, 'application/json');
    echo "=== JSON Extraction ===\n";
    echo "Content:\n{$result->content}\n";
    echo "Fields count: " . $result->metadata['fields_count'] . "\n";
    echo "Tables found: " . count($result->tables) . "\n";
    if (!empty($result->tables)) {
        echo "First table:\n{$result->tables[0]->markdown}\n";
    }
    echo "\n";
} catch (Exception $e) {
    echo "Error: {$e->getMessage()}\n";
}


echo "=== Registered Extractors ===\n";
$extractors = ExtractorRegistry::list();
foreach ($extractors as $mimeType) {
    echo "- {$mimeType}\n";
}
echo "\n";

echo "=== Testing Custom Extractor ===\n";
$testData = "TITLE: Test\nContent here";
try {
    $success = ExtractorRegistry::test('text/x-custom', $testData);
    echo "Test " . ($success ? "PASSED" : "FAILED") . "\n";
} catch (Exception $e) {
    echo "Test error: {$e->getMessage()}\n";
}
echo "\n";


ExtractorRegistry::unregister('text/simple');
echo "Unregistered 'text/simple' extractor\n";
