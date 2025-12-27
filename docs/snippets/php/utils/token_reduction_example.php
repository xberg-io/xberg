```php
<?php

declare(strict_types=1);

/**
 * Token Reduction Example
 *
 * Practical example of using token reduction to fit documents within token limits.
 * Demonstrates tracking reduction statistics and optimizing for LLM usage.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\TokenReductionConfig;

$config = new ExtractionConfig(
    tokenReduction: new TokenReductionConfig(
        mode: 'moderate',
        preserveMarkdown: true
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('verbose_document.pdf');

echo "Token Reduction Example:\n";
echo str_repeat('=', 60) . "\n";
echo "Document: verbose_document.pdf\n\n";

if (isset($result->metadata['original_token_count'])) {
    $originalTokens = $result->metadata['original_token_count'];
    $reducedTokens = $result->metadata['token_count'];
    $reductionRatio = $result->metadata['token_reduction_ratio'];

    echo "Token Reduction Statistics:\n";
    echo str_repeat('-', 40) . "\n";
    echo sprintf("  Before:    %s tokens\n", number_format($originalTokens));
    echo sprintf("  After:     %s tokens\n", number_format($reducedTokens));
    echo sprintf("  Reduction: %.1f%%\n", $reductionRatio * 100);
    echo sprintf("  Saved:     %s tokens\n\n", number_format($originalTokens - $reducedTokens));

    $beforeBar = str_repeat('█', (int)($originalTokens / 100));
    $afterBar = str_repeat('█', (int)($reducedTokens / 100));

    echo "Visual comparison (each █ = ~100 tokens):\n";
    echo "  Before: $beforeBar\n";
    echo "  After:  $afterBar\n\n";
}

echo "Content Analysis:\n";
echo str_repeat('-', 40) . "\n";
echo "  Content length: " . strlen($result->content) . " characters\n";
echo "  First 200 chars: " . substr($result->content, 0, 200) . "...\n\n";

$documents = [
    'long_article.pdf',
    'research_paper.pdf',
    'technical_doc.pdf',
];

echo "Batch Token Reduction:\n";
echo str_repeat('=', 60) . "\n";

$batchConfig = new ExtractionConfig(
    tokenReduction: new TokenReductionConfig(
        mode: 'moderate',
        preserveImportantWords: true,
        preserveMarkdown: true
    )
);

$kreuzberg = new Kreuzberg($batchConfig);
$totalOriginal = 0;
$totalReduced = 0;

foreach ($documents as $document) {
    if (!file_exists($document)) {
        echo basename($document) . ": File not found\n\n";
        continue;
    }

    $result = $kreuzberg->extractFile($document);

    $originalTokens = $result->metadata['original_token_count'] ?? 0;
    $reducedTokens = $result->metadata['token_count'] ?? 0;
    $reductionRatio = $result->metadata['token_reduction_ratio'] ?? 0;

    $totalOriginal += $originalTokens;
    $totalReduced += $reducedTokens;

    echo basename($document) . ":\n";
    echo sprintf("  Original: %s tokens\n", number_format($originalTokens));
    echo sprintf("  Reduced:  %s tokens\n", number_format($reducedTokens));
    echo sprintf("  Saved:    %.1f%%\n\n", $reductionRatio * 100);
}

if ($totalOriginal > 0) {
    $overallReduction = (($totalOriginal - $totalReduced) / $totalOriginal) * 100;

    echo "Overall Statistics:\n";
    echo str_repeat('-', 40) . "\n";
    echo sprintf("  Total original: %s tokens\n", number_format($totalOriginal));
    echo sprintf("  Total reduced:  %s tokens\n", number_format($totalReduced));
    echo sprintf("  Overall saving: %.1f%%\n\n", $overallReduction);
}

function fitWithinTokenLimit(
    string $filePath,
    int $maxTokens,
    Kreuzberg $kreuzberg
): ?array {
    $modes = ['light', 'moderate', 'aggressive'];

    foreach ($modes as $mode) {
        $config = new ExtractionConfig(
            tokenReduction: new TokenReductionConfig(
                mode: $mode,
                preserveImportantWords: true
            )
        );

        $kreuzbergWithMode = new Kreuzberg($config);
        $result = $kreuzbergWithMode->extractFile($filePath);

        $tokens = $result->metadata['token_count'] ?? strlen($result->content);

        if ($tokens <= $maxTokens) {
            return [
                'mode' => $mode,
                'tokens' => $tokens,
                'result' => $result,
                'fits' => true,
            ];
        }
    }

    $config = new ExtractionConfig(
        tokenReduction: new TokenReductionConfig(
            mode: 'aggressive',
            preserveImportantWords: true
        )
    );

    $kreuzbergWithMode = new Kreuzberg($config);
    $result = $kreuzbergWithMode->extractFile($filePath);
    $tokens = $result->metadata['token_count'] ?? strlen($result->content);

    return [
        'mode' => 'aggressive',
        'tokens' => $tokens,
        'result' => $result,
        'fits' => false,
    ];
}

echo "Fitting Document to Token Limit:\n";
echo str_repeat('=', 60) . "\n";

$tokenLimit = 8000;
$testFile = 'large_document.pdf';

if (file_exists($testFile)) {
    $fitResult = fitWithinTokenLimit($testFile, $tokenLimit, $kreuzberg);

    echo "Target limit: " . number_format($tokenLimit) . " tokens\n";
    echo "Reduction mode used: {$fitResult['mode']}\n";
    echo "Final token count: " . number_format($fitResult['tokens']) . "\n";

    if ($fitResult['fits']) {
        echo "Status: ✓ Successfully fits within limit\n";
        $remaining = $tokenLimit - $fitResult['tokens'];
        echo "Tokens remaining: " . number_format($remaining) . "\n";
    } else {
        echo "Status: ✗ Still exceeds limit\n";
        $excess = $fitResult['tokens'] - $tokenLimit;
        echo "Tokens over limit: " . number_format($excess) . "\n";
        echo "Suggestion: Consider chunking the document\n";
    }

    echo "\n";
}

function calculateCostSavings(
    int $originalTokens,
    int $reducedTokens,
    float $pricePerMillion = 0.50
): array {
    $originalCost = ($originalTokens / 1_000_000) * $pricePerMillion;
    $reducedCost = ($reducedTokens / 1_000_000) * $pricePerMillion;
    $savings = $originalCost - $reducedCost;
    $savingsPercent = ($savings / max($originalCost, 0.000001)) * 100;

    return [
        'original_cost' => $originalCost,
        'reduced_cost' => $reducedCost,
        'savings' => $savings,
        'savings_percent' => $savingsPercent,
    ];
}

if ($totalOriginal > 0 && $totalReduced > 0) {
    $savings = calculateCostSavings($totalOriginal, $totalReduced);

    echo "Cost Analysis:\n";
    echo str_repeat('=', 60) . "\n";
    echo "Price: $0.50 per million tokens (example)\n\n";
    echo sprintf("  Original cost: $%.6f\n", $savings['original_cost']);
    echo sprintf("  Reduced cost:  $%.6f\n", $savings['reduced_cost']);
    echo sprintf("  Savings:       $%.6f (%.1f%%)\n\n", $savings['savings'], $savings['savings_percent']);

    $documentsPerDay = 100;
    $daysPerYear = 365;
    $annualSavings = $savings['savings'] * $documentsPerDay * $daysPerYear;

    echo "Projected Annual Savings:\n";
    echo "  Documents per day: $documentsPerDay\n";
    echo "  Annual savings: $" . number_format($annualSavings, 2) . "\n";
}
```
