```php title="keyword_config.php"
<?php

declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\ExtractInput;
use Xberg\ExtractionConfig;
use Xberg\Xberg;

function keywordConfig(array $keywords): ExtractionConfig
{
    return ExtractionConfig::from_json(json_encode([
        'keywords' => $keywords,
    ], JSON_THROW_ON_ERROR));
}

function extractFirst(string $uri, ExtractionConfig $config): object
{
    $output = Xberg::extract(ExtractInput::fromUri($uri), $config);
    return $output->results[0];
}

$config = keywordConfig([
    'algorithm' => 'yake',
    'maxKeywords' => 10,
    'minScore' => 0.0,
    'language' => 'en',
]);

$result = extractFirst('article.pdf', $config);

echo "Top Keywords:\n";
echo str_repeat('=', 40) . "\n";
foreach ($result->extractedKeywords ?? [] as $keyword) {
    echo "  - {$keyword->text} ({$keyword->score})\n";
}
echo "\n";

$detailedConfig = keywordConfig([
    'algorithm' => 'yake',
    'maxKeywords' => 25,
    'minScore' => 0.0,
    'language' => 'en',
]);

$result = extractFirst('research_paper.pdf', $detailedConfig);
$keywords = $result->extractedKeywords ?? [];

echo "Detailed keyword analysis:\n";
echo "Total keywords: " . count($keywords) . "\n";

if ($keywords !== []) {
    $grouped = [];
    foreach ($keywords as $keyword) {
        $first = strtoupper($keyword->text[0]);
        $grouped[$first][] = $keyword->text;
    }

    foreach ($grouped as $letter => $items) {
        echo "\n$letter:\n";
        foreach ($items as $item) {
            echo "  - $item\n";
        }
    }
}

$files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf'];
$allKeywords = [];

foreach ($files as $file) {
    if (!file_exists($file)) {
        continue;
    }

    $result = extractFirst($file, $detailedConfig);
    foreach ($result->extractedKeywords ?? [] as $keyword) {
        $allKeywords[$keyword->text] = ($allKeywords[$keyword->text] ?? 0) + 1;
    }
}

arsort($allKeywords);
echo "\n\nMost common keywords across documents:\n";
$rank = 0;
foreach ($allKeywords as $keyword => $frequency) {
    if ($rank++ >= 10) {
        break;
    }
    echo sprintf("  %2d. %-30s (appears in %d documents)\n", $rank, $keyword, $frequency);
}
```
