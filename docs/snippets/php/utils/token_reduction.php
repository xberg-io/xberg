```php title="token_reduction.php"
<?php

declare(strict_types=1);

/**
 * Token Reduction Configuration
 *
 * Configure token reduction to compress extracted content while preserving meaning.
 * Useful for reducing token costs in LLM applications and staying within token limits.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\TokenReductionConfig;

$config = new ExtractionConfig(
    tokenReduction: new TokenReductionConfig(
        mode: 'moderate',
        preserveImportantWords: true
    )
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Token Reduction Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Content length: " . strlen($result->getContent()) . " characters\n\n";

if (isset($result->metadata['original_token_count'])) {
    $originalTokens = $result->metadata['original_token_count'];
    $reducedTokens = $result->metadata['token_count'] ?? strlen($result->getContent());
    $reductionRatio = $result->metadata['token_reduction_ratio'] ?? 0;

    echo "Token Reduction Statistics:\n";
    echo str_repeat('-', 40) . "\n";
    echo "  Original tokens: " . number_format($originalTokens) . "\n";
    echo "  Reduced tokens: " . number_format($reducedTokens) . "\n";
    echo "  Reduction: " . number_format($reductionRatio * 100, 1) . "%\n";
    echo "  Tokens saved: " . number_format($originalTokens - $reducedTokens) . "\n\n";
}

$modes = [
    'light' => 'Light reduction - minimal changes',
    'moderate' => 'Moderate reduction - balanced',
    'aggressive' => 'Aggressive reduction - maximum compression',
];

echo "Token Reduction Mode Comparison:\n";
echo str_repeat('=', 60) . "\n";

$comparisonResults = [];

foreach ($modes as $mode => $description) {
    $modeConfig = new ExtractionConfig(
        tokenReduction: new TokenReductionConfig(
            mode: $mode,
            preserveImportantWords: true
        )
    );

    $output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('sample.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

    $contentLength = strlen($result->getContent());
    $tokenCount = $result->metadata['token_count'] ?? $contentLength;

    $comparisonResults[$mode] = [
        'length' => $contentLength,
        'tokens' => $tokenCount,
        'content' => substr($result->getContent(), 0, 100),
    ];

    echo "$mode mode:\n";
    echo "  Description: $description\n";
    echo "  Content length: " . number_format($contentLength) . " characters\n";
    echo "  Estimated tokens: " . number_format($tokenCount) . "\n";
    echo "  Preview: " . substr($result->getContent(), 0, 80) . "...\n\n";
}

if (count($comparisonResults) > 1) {
    $lightLength = $comparisonResults['light']['length'] ?? 0;
    $aggressiveLength = $comparisonResults['aggressive']['length'] ?? 0;

    if ($lightLength > 0) {
        $savings = (($lightLength - $aggressiveLength) / $lightLength) * 100;

        echo "Comparison Summary:\n";
        echo str_repeat('-', 40) . "\n";
        echo "Aggressive vs Light mode saves: " . number_format($savings, 1) . "%\n\n";
    }
}

$advancedConfig = new ExtractionConfig(
    tokenReduction: new TokenReductionConfig(
        mode: 'moderate',
        preserveImportantWords: true,
        preserveMarkdown: true,
        preserveNumbers: true,
        removeStopWords: true
    )
);

$output = \Xberg\Xberg::extract(\Xberg\ExtractInput::uri('verbose_document.pdf'), $config ?? \Xberg\ExtractionConfig::default());
$result = $output->results[0];

echo "Advanced Token Reduction:\n";
echo str_repeat('=', 60) . "\n";
echo "Configuration:\n";
echo "  - Preserve important words: Yes\n";
echo "  - Preserve markdown: Yes\n";
echo "  - Preserve numbers: Yes\n";
echo "  - Remove stop words: Yes\n\n";

echo "Result:\n";
echo "  Content length: " . strlen($result->getContent()) . " characters\n";

if (isset($result->metadata['token_reduction_ratio'])) {
    echo "  Reduction ratio: " . number_format($result->metadata['token_reduction_ratio'] * 100, 1) . "%\n";
}

echo "\n";

function estimateTokenCost(int $tokens, float $pricePerMillion = 0.50): float
{
    return ($tokens / 1_000_000) * $pricePerMillion;
}

echo "Cost Estimation (based on reduction):\n";
echo str_repeat('=', 60) . "\n";

foreach ($comparisonResults as $mode => $data) {
    $tokens = $data['tokens'];
    $cost = estimateTokenCost($tokens);

    echo ucfirst($mode) . " mode:\n";
    echo "  Tokens: " . number_format($tokens) . "\n";
    echo "  Estimated cost: $" . number_format($cost, 4) . "\n\n";
}

function chooseReductionMode(int $maxTokens, int $estimatedTokens): string
{
    $ratio = $estimatedTokens / $maxTokens;

    return match(true) {
        $ratio <= 1.0 => 'none',      
        $ratio <= 1.3 => 'light',     
        $ratio <= 1.7 => 'moderate',  
        default => 'aggressive',      
    };
}

$maxTokenLimit = 8000;
$documentTokens = 12000;

$recommendedMode = chooseReductionMode($maxTokenLimit, $documentTokens);

echo "Reduction Mode Recommendation:\n";
echo str_repeat('=', 60) . "\n";
echo "Document tokens: " . number_format($documentTokens) . "\n";
echo "Token limit: " . number_format($maxTokenLimit) . "\n";
echo "Recommended mode: $recommendedMode\n";
echo "Reason: " . ($documentTokens > $maxTokenLimit
    ? "Document exceeds limit by " . number_format($documentTokens - $maxTokenLimit) . " tokens"
    : "Document within limits") . "\n";
```
