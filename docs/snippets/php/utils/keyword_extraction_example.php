```php title="keyword_extraction_example.php"
<?php

declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\ExtractInput;
use Xberg\ExtractionConfig;
use Xberg\Xberg;

function keywordConfig(string $algorithm, int $maxKeywords, float $minScore): ExtractionConfig
{
    return ExtractionConfig::from_json(json_encode([
        'keywords' => [
            'algorithm' => $algorithm,
            'maxKeywords' => $maxKeywords,
            'minScore' => $minScore,
        ],
    ], JSON_THROW_ON_ERROR));
}

function extractFirst(string $uri, ExtractionConfig $config): object
{
    $output = Xberg::extract(ExtractInput::fromUri($uri), $config);
    return $output->results[0];
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
        $scores[$category] = 0.0;

        foreach ($keywords as $keyword) {
            $keywordText = strtolower($keyword->text);

            foreach ($terms as $term) {
                if (str_contains($keywordText, $term)) {
                    $scores[$category] += $keyword->score;
                }
            }
        }
    }

    arsort($scores);
    return array_key_first($scores) ?? 'uncategorized';
}

$config = keywordConfig('yake', 10, 0.3);
$result = extractFirst('research_paper.pdf', $config);
$keywords = $result->extractedKeywords ?? [];

echo "Keyword Extraction Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Document: research_paper.pdf\n";
echo "Content length: " . strlen($result->content) . " characters\n\n";

if ($keywords !== []) {
    echo "Extracted Keywords:\n";
    echo str_repeat('-', 40) . "\n";

    foreach ($keywords as $keyword) {
        echo sprintf("  %-30s  Score: %.3f\n", $keyword->text, $keyword->score);
    }
    echo "\n";
} else {
    echo "No keywords extracted. Try adjusting minScore or maxKeywords.\n\n";
}

$algorithms = [
    'YAKE' => 'yake',
    'RAKE' => 'rake',
];

echo "Algorithm Comparison:\n";
echo str_repeat('=', 60) . "\n";

foreach ($algorithms as $name => $algorithm) {
    $result = extractFirst('article.pdf', keywordConfig($algorithm, 5, 0.2));
    $keywords = $result->extractedKeywords ?? [];

    echo "$name algorithm:\n";

    if ($keywords !== []) {
        foreach ($keywords as $keyword) {
            echo "  - {$keyword->text} ({$keyword->score})\n";
        }
    } else {
        echo "  No keywords extracted\n";
    }

    echo "\n";
}

if ($keywords !== []) {
    $category = categorizeDocument($keywords);
    echo "Document Category: " . ucfirst($category) . "\n\n";
}

$documents = [
    'tech_article.pdf',
    'business_report.pdf',
    'research_paper.pdf',
];

$batchConfig = keywordConfig('yake', 8, 0.25);

echo "Batch Keyword Extraction:\n";
echo str_repeat('=', 60) . "\n";

foreach ($documents as $document) {
    if (!file_exists($document)) {
        echo "$document: File not found\n\n";
        continue;
    }

    $result = extractFirst($document, $batchConfig);
    $keywords = $result->extractedKeywords ?? [];

    echo basename($document) . ":\n";

    if ($keywords !== []) {
        $topKeywords = array_slice($keywords, 0, 5);
        $keywordTexts = array_map(static fn(object $keyword): string => $keyword->text, $topKeywords);
        echo "  Top keywords: " . implode(', ', $keywordTexts) . "\n";
        echo "  Category: " . ucfirst(categorizeDocument($keywords)) . "\n";
    } else {
        echo "  No keywords extracted\n";
    }

    echo "\n";
}
```
