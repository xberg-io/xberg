```php
<?php

declare(strict_types=1);

/**
 * Metadata Extraction
 *
 * Extract and process document metadata including title, author,
 * creation date, keywords, and custom properties.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use function Kreuzberg\extract_file;

$result = extract_file('document.pdf');
$metadata = $result->metadata;

echo "Document Metadata:\n";
echo str_repeat('=', 60) . "\n";
echo "Title: " . ($metadata->title ?? 'N/A') . "\n";
echo "Authors: " . (isset($metadata->authors) ? implode(', ', $metadata->authors) : 'N/A') . "\n";
echo "Subject: " . ($metadata->subject ?? 'N/A') . "\n";
echo "Created By: " . ($metadata->createdBy ?? 'N/A') . "\n";
echo "Producer: " . ($metadata->producer ?? 'N/A') . "\n";
echo "Created: " . ($metadata->createdAt ?? 'N/A') . "\n";
echo "Modified: " . ($metadata->modifiedAt ?? 'N/A') . "\n";
echo "Page Count: " . ($metadata->pageCount ?? 'N/A') . "\n";
echo "Keywords: " . implode(', ', $metadata->keywords ?? []) . "\n";
echo "Language: " . ($metadata->language ?? 'N/A') . "\n\n";

$files = glob('documents/*.{pdf,docx,xlsx}', GLOB_BRACE);
$metadataCollection = [];

foreach ($files as $file) {
    $result = extract_file($file);
    $metadataCollection[] = [
        'file' => basename($file),
        'title' => $result->metadata->title ?? 'Untitled',
        'author' => isset($result->metadata->authors) ? implode(', ', $result->metadata->authors) : 'Unknown',
        'created' => $result->metadata->createdAt ?? 'Unknown',
        'pages' => $result->metadata->pageCount ?? 0,
        'size' => filesize($file),
    ];
}

echo "Metadata Collection:\n";
echo str_repeat('=', 60) . "\n";
foreach ($metadataCollection as $meta) {
    echo "{$meta['file']}:\n";
    echo "  Title: {$meta['title']}\n";
    echo "  Author: {$meta['author']}\n";
    echo "  Created: {$meta['created']}\n";
    echo "  Pages: {$meta['pages']}\n";
    echo "  Size: " . number_format($meta['size'] / 1024, 2) . " KB\n\n";
}

function searchByAuthor(array $collection, string $author): array
{
    return array_filter($collection, function ($meta) use ($author) {
        return stripos($meta['author'], $author) !== false;
    });
}

function searchByDateRange(array $collection, string $start, string $end): array
{
    return array_filter($collection, function ($meta) use ($start, $end) {
        $created = $meta['created'];
        if ($created === 'Unknown') {
            return false;
        }
        $dateOnly = substr($created, 0, 10);
        return $dateOnly >= $start && $dateOnly <= $end;
    });
}

$johnDocs = searchByAuthor($metadataCollection, 'John');
echo "Documents by John: " . count($johnDocs) . "\n";

$recentDocs = searchByDateRange($metadataCollection, '2024-01-01', '2024-12-31');
echo "Documents from 2024: " . count($recentDocs) . "\n\n";

function generateCatalog(array $collection): string
{
    $html = "<html><head><title>Document Catalog</title></head><body>\n";
    $html .= "<h1>Document Catalog</h1>\n";
    $html .= "<table border='1'>\n";
    $html .= "<tr><th>File</th><th>Title</th><th>Author</th><th>Created</th><th>Pages</th></tr>\n";

    foreach ($collection as $meta) {
        $html .= "<tr>";
        $html .= "<td>" . htmlspecialchars($meta['file']) . "</td>";
        $html .= "<td>" . htmlspecialchars($meta['title']) . "</td>";
        $html .= "<td>" . htmlspecialchars($meta['author']) . "</td>";
        $html .= "<td>" . htmlspecialchars($meta['created']) . "</td>";
        $html .= "<td>" . htmlspecialchars((string)$meta['pages']) . "</td>";
        $html .= "</tr>\n";
    }

    $html .= "</table>\n</body></html>";
    return $html;
}

$catalog = generateCatalog($metadataCollection);
file_put_contents('catalog.html', $catalog);
echo "Catalog saved to: catalog.html\n";

function exportMetadataToCSV(array $collection, string $filename): void
{
    $fp = fopen($filename, 'w');

    fputcsv($fp, ['File', 'Title', 'Author', 'Created', 'Pages', 'Size (KB)']);

    foreach ($collection as $meta) {
        fputcsv($fp, [
            $meta['file'],
            $meta['title'],
            $meta['author'],
            $meta['created'],
            $meta['pages'],
            number_format($meta['size'] / 1024, 2),
        ]);
    }

    fclose($fp);
}

exportMetadataToCSV($metadataCollection, 'metadata.csv');
echo "Metadata exported to: metadata.csv\n";

$totalPages = array_sum(array_column($metadataCollection, 'pages'));
$totalSize = array_sum(array_column($metadataCollection, 'size'));
$authors = array_unique(array_column($metadataCollection, 'author'));

echo "\nCollection Statistics:\n";
echo str_repeat('=', 60) . "\n";
echo "Total documents: " . count($metadataCollection) . "\n";
echo "Total pages: " . number_format($totalPages) . "\n";
echo "Total size: " . number_format($totalSize / 1024 / 1024, 2) . " MB\n";
echo "Unique authors: " . count($authors) . "\n";
echo "Average pages per document: " . number_format($totalPages / count($metadataCollection), 1) . "\n";

$byAuthor = [];
foreach ($metadataCollection as $meta) {
    $author = $meta['author'];
    if (!isset($byAuthor[$author])) {
        $byAuthor[$author] = [];
    }
    $byAuthor[$author][] = $meta;
}

echo "\nDocuments by Author:\n";
echo str_repeat('=', 60) . "\n";
foreach ($byAuthor as $author => $docs) {
    echo "$author: " . count($docs) . " documents\n";
}

function validateMetadata(array $meta): array
{
    $issues = [];

    if (empty($meta['title']) || $meta['title'] === 'Untitled') {
        $issues[] = 'Missing title';
    }

    if (empty($meta['author']) || $meta['author'] === 'Unknown') {
        $issues[] = 'Missing author';
    }

    if (empty($meta['created']) || $meta['created'] === 'Unknown') {
        $issues[] = 'Missing creation date';
    }

    if ($meta['pages'] === 0) {
        $issues[] = 'Invalid page count';
    }

    return $issues;
}

echo "\nMetadata Quality Check:\n";
echo str_repeat('=', 60) . "\n";

$incomplete = 0;
foreach ($metadataCollection as $meta) {
    $issues = validateMetadata($meta);
    if (!empty($issues)) {
        $incomplete++;
        echo "{$meta['file']}: " . implode(', ', $issues) . "\n";
    }
}

echo "\nIncomplete metadata: $incomplete/" . count($metadataCollection) . " documents\n";
```
