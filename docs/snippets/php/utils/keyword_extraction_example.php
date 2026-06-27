```php title="keyword_extraction_example.php"
<?php

declare(strict_types=1);

/**
 * Keyword Extraction Example
 *
 * Extract keywords from documents using various algorithms.
 * Demonstrates automatic keyword detection for document analysis and indexing.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\KeywordConfig;
use Xberg\Enums\KeywordAlgorithm;

$config = new ExtractionConfig(
    keywords: new KeywordConfig(
        algorithm: KeywordAlgorithm::YAKE,
        maxKeywords: 10,
        minScore: 0.3
    )
);

$xberg = new Xberg($config);
$result = $xberg->extract('research_paper.pdf');

echo "Keyword Extraction Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Document: research_paper.pdf\n";
echo "Content length: " . strlen($result->content) . " characters\n\n";

$keywords = $result->metadata['keywords'] ?? [];

if (!empty($keywords)) {
    echo "Extracted Keywords:\n";
    echo str_repeat('-', 40) . "\n";

    foreach ($keywords as $keyword) {
        $text = $keyword['text'] ?? '';
        $score = $keyword['score'] ?? 0.0;
        $frequency = $keyword['frequency'] ?? null;

        echo sprintf("  %-30s  Score: %.3f", $text, $score);

        if ($frequency !== null) {
            echo sprintf("  (appears %d times)", $frequency);
        }

        echo "\n";
    }
    echo "\n";
} else {
    echo "No keywords extracted. Try adjusting minScore or maxKeywords.\n\n";
}

$algorithms = [
    'YAKE' => KeywordAlgorithm::YAKE,
    'TextRank' => KeywordAlgorithm::TEXT_RANK,
    'TF-IDF' => KeywordAlgorithm::TF_IDF,
];

echo "Algorithm Comparison:\n";
echo str_repeat('=', 60) . "\n";

foreach ($algorithms as $name => $algorithm) {
    $algoConfig = new ExtractionConfig(
        keywords: new KeywordConfig(
            algorithm: $algorithm,
            maxKeywords: 5,
            minScore: 0.2
        )
    );

    $xberg = new Xberg($algoConfig);
    $result = $xberg->extract('article.pdf');

    $keywords = $result->metadata['keywords'] ?? [];

    echo "$name algorithm:\n";

    if (!empty($keywords)) {
        foreach ($keywords as $keyword) {
            echo "  - {$keyword['text']} ({$keyword['score']})\n";
        }
    } else {
        echo "  No keywords extracted\n";
    }

    echo "\n";
}

function categorizeDocument(array $keywords): string
{
    $categories = [
        'technical' => ['algorithm', 'system', 'implementation', 'performance', 'architecture'],
        'business' => ['revenue', 'market', 'customer', 'strategy', 'investment'],
        'scientific' => ['research', 'study', 'analysis', 'experiment', 'hypothesis'],
        'legal' => ['contract', 'agreement', 'liability', 'clause', 'provision'],
    ];

    $scores = [];
    foreach ($categories as $category => $terms) {
        $scores[$category] = 0;

        foreach ($keywords as $keyword) {
            $keywordText = strtolower($keyword['text'] ?? '');
            $keywordScore = $keyword['score'] ?? 0.0;

            foreach ($terms as $term) {
                if (str_contains($keywordText, $term)) {
                    $scores[$category] += $keywordScore;
                }
            }
        }
    }

    arsort($scores);
    $topCategory = array_key_first($scores);

    return $topCategory ?? 'uncategorized';
}

if (!empty($keywords)) {
    $category = categorizeDocument($keywords);
    echo "Document Category: " . ucfirst($category) . "\n\n";
}

$documents = [
    'tech_article.pdf',
    'business_report.pdf',
    'research_paper.pdf',
];

$keywordConfig = new ExtractionConfig(
    keywords: new KeywordConfig(
        algorithm: KeywordAlgorithm::YAKE,
        maxKeywords: 8,
        minScore: 0.25
    )
);

$xberg = new Xberg($keywordConfig);

echo "Batch Keyword Extraction:\n";
echo str_repeat('=', 60) . "\n";

foreach ($documents as $document) {
    if (!file_exists($document)) {
        echo "$document: File not found\n\n";
        continue;
    }

    $result = $xberg->extract($document);
    $keywords = $result->metadata['keywords'] ?? [];

    echo basename($document) . ":\n";

    if (!empty($keywords)) {
        $topKeywords = array_slice($keywords, 0, 5);
        $keywordTexts = array_column($topKeywords, 'text');
        echo "  Top keywords: " . implode(', ', $keywordTexts) . "\n";

        $category = categorizeDocument($keywords);
        echo "  Category: " . ucfirst($category) . "\n";
    } else {
        echo "  No keywords extracted\n";
    }

    echo "\n";
}

$keywordIndex = [];

foreach ($documents as $document) {
    if (!file_exists($document)) {
        continue;
    }

    $result = $xberg->extract($document);
    $keywords = $result->metadata['keywords'] ?? [];

    foreach ($keywords as $keyword) {
        $text = strtolower($keyword['text'] ?? '');
        if (!isset($keywordIndex[$text])) {
            $keywordIndex[$text] = [];
        }
        $keywordIndex[$text][] = basename($document);
    }
}

echo "Keyword Index (for search):\n";
echo str_repeat('=', 60) . "\n";
foreach (array_slice($keywordIndex, 0, 10) as $keyword => $docs) {
    echo "$keyword: " . implode(', ', array_unique($docs)) . "\n";
}
```
