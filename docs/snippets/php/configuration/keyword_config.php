```php
<?php

declare(strict_types=1);

/**
 * KeywordConfig - Keyword Extraction
 *
 * Automatically extract keywords and key phrases from documents.
 * Useful for document categorization, search indexing, and summarization.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\KeywordConfig;

$config = new ExtractionConfig(
    keyword: new KeywordConfig(
        maxKeywords: 10,
        minScore: 0.0,
        language: 'en'
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('article.pdf');

echo "Top Keywords:\n";
echo str_repeat('=', 40) . "\n";
foreach ($result->metadata->keywords ?? [] as $keyword) {
    echo "  • $keyword\n";
}
echo "\n";

$detailedConfig = new ExtractionConfig(
    keyword: new KeywordConfig(
        maxKeywords: 25,
        minScore: 0.0,
        language: 'en'
    )
);

$kreuzberg = new Kreuzberg($detailedConfig);
$result = $kreuzberg->extractFile('research_paper.pdf');

echo "Detailed keyword analysis:\n";
echo "Total keywords: " . count($result->metadata->keywords ?? []) . "\n";

if (!empty($result->metadata->keywords)) {
    $grouped = [];
    foreach ($result->metadata->keywords as $keyword) {
        $first = strtoupper($keyword[0]);
        if (!isset($grouped[$first])) {
            $grouped[$first] = [];
        }
        $grouped[$first][] = $keyword;
    }

    foreach ($grouped as $letter => $keywords) {
        echo "\n$letter:\n";
        foreach ($keywords as $keyword) {
            echo "  - $keyword\n";
        }
    }
}

$files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf'];
$allKeywords = [];

foreach ($files as $file) {
    if (!file_exists($file)) continue;

    $result = $kreuzberg->extractFile($file);
    foreach ($result->metadata->keywords ?? [] as $keyword) {
        if (!isset($allKeywords[$keyword])) {
            $allKeywords[$keyword] = 0;
        }
        $allKeywords[$keyword]++;
    }
}

arsort($allKeywords);
echo "\n\nMost common keywords across documents:\n";
$count = 0;
foreach ($allKeywords as $keyword => $frequency) {
    if ($count++ >= 10) break;
    echo sprintf("  %2d. %-30s (appears in %d documents)\n",
        $count, $keyword, $frequency);
}

$categoryKeywords = [
    'technology' => ['software', 'computer', 'algorithm', 'data', 'system'],
    'business' => ['market', 'revenue', 'sales', 'customer', 'profit'],
    'science' => ['research', 'experiment', 'hypothesis', 'analysis', 'study'],
];

$docKeywords = $result->metadata->keywords ?? [];
$scores = [];

foreach ($categoryKeywords as $category => $terms) {
    $score = 0;
    foreach ($terms as $term) {
        if (in_array($term, $docKeywords, true)) {
            $score++;
        }
    }
    $scores[$category] = $score;
}

arsort($scores);
$topCategory = array_key_first($scores);
echo "\nDocument category: $topCategory (score: {$scores[$topCategory]})\n";
```
