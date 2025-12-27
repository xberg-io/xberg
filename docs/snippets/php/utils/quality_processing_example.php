```php
<?php

declare(strict_types=1);

/**
 * Quality Processing Example
 *
 * Enable quality processing to assess and improve extraction quality.
 * Useful for detecting low-quality scans and suggesting improvements.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;

$config = new ExtractionConfig(
    enableQualityProcessing: true
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('scanned_document.pdf');

echo "Quality Processing Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Document: scanned_document.pdf\n";
echo "Content length: " . strlen($result->content) . " characters\n\n";

$qualityScore = $result->metadata['quality_score'] ?? null;

if ($qualityScore !== null) {
    echo "Quality Score: " . number_format($qualityScore, 2) . "\n";
    echo "Rating: ";

    if ($qualityScore >= 0.8) {
        echo "Excellent\n";
        echo "Status: ✓ Ready for production use\n";
    } elseif ($qualityScore >= 0.6) {
        echo "Good\n";
        echo "Status: ✓ Acceptable quality\n";
    } elseif ($qualityScore >= 0.5) {
        echo "Fair\n";
        echo "Status: ⚠ May require review\n";
    } else {
        echo "Poor\n";
        echo "Status: ✗ Requires attention\n";
    }

    echo "\n";

    if ($qualityScore < 0.5) {
        echo "Recommendations for Improvement:\n";
        echo str_repeat('-', 40) . "\n";
        echo "1. Re-scan with higher DPI (300+ recommended)\n";
        echo "2. Ensure original is clean and well-lit\n";
        echo "3. Adjust OCR preprocessing settings:\n";
        echo "   - Enable denoising\n";
        echo "   - Enable deskewing\n";
        echo "   - Increase contrast enhancement\n";
        echo "4. Try different binarization methods\n";
        echo "5. Consider manual review and correction\n\n";
    }
} else {
    echo "Quality score not available.\n";
    echo "Enable quality processing in configuration.\n\n";
}

if (isset($result->metadata['ocr_confidence'])) {
    $ocrConfidence = $result->metadata['ocr_confidence'];
    echo "OCR Confidence: " . number_format($ocrConfidence * 100, 1) . "%\n\n";

    if ($ocrConfidence < 0.7) {
        echo "⚠ Low OCR confidence detected.\n";
        echo "The extracted text may contain errors.\n\n";
    }
}

if (isset($result->metadata['quality_metrics'])) {
    echo "Detailed Quality Metrics:\n";
    echo str_repeat('-', 40) . "\n";

    $metrics = $result->metadata['quality_metrics'];

    foreach ($metrics as $metric => $value) {
        $formattedValue = is_numeric($value)
            ? number_format($value, 3)
            : $value;

        echo sprintf("  %-25s: %s\n", ucwords(str_replace('_', ' ', $metric)), $formattedValue);
    }

    echo "\n";
}

$documents = [
    'high_quality_scan.pdf',
    'medium_quality_scan.pdf',
    'low_quality_scan.pdf',
];

echo "Batch Quality Analysis:\n";
echo str_repeat('=', 60) . "\n";

$qualityConfig = new ExtractionConfig(
    enableQualityProcessing: true
);

$kreuzberg = new Kreuzberg($qualityConfig);
$qualityResults = [];

foreach ($documents as $document) {
    if (!file_exists($document)) {
        echo basename($document) . ": File not found\n\n";
        continue;
    }

    $result = $kreuzberg->extractFile($document);
    $score = $result->metadata['quality_score'] ?? 0.0;

    $qualityResults[$document] = [
        'score' => $score,
        'content_length' => strlen($result->content),
        'result' => $result,
    ];

    echo basename($document) . ":\n";
    echo "  Quality score: " . number_format($score, 2) . "\n";
    echo "  Content length: " . strlen($result->content) . " chars\n";

    $indicator = match(true) {
        $score >= 0.8 => '✓ Excellent',
        $score >= 0.6 => '✓ Good',
        $score >= 0.5 => '⚠ Fair',
        default => '✗ Poor',
    };

    echo "  Status: $indicator\n\n";
}

if (!empty($qualityResults)) {
    $scores = array_column($qualityResults, 'score');
    $avgScore = array_sum($scores) / count($scores);
    $maxScore = max($scores);
    $minScore = min($scores);

    echo "Quality Statistics:\n";
    echo str_repeat('-', 40) . "\n";
    echo "  Average: " . number_format($avgScore, 2) . "\n";
    echo "  Highest: " . number_format($maxScore, 2) . "\n";
    echo "  Lowest:  " . number_format($minScore, 2) . "\n\n";

    $lowQualityDocs = array_filter(
        $qualityResults,
        fn($result) => $result['score'] < 0.5
    );

    if (!empty($lowQualityDocs)) {
        echo "Documents Requiring Attention:\n";
        echo str_repeat('-', 40) . "\n";

        foreach ($lowQualityDocs as $doc => $data) {
            echo "  - " . basename($doc) . " (score: " . number_format($data['score'], 2) . ")\n";
        }

        echo "\n";
    }
}

function needsReprocessing(float $qualityScore, int $contentLength): bool
{
    return $qualityScore < 0.5 || $contentLength < 100;
}

function routeDocumentByQuality(string $filePath, float $qualityScore): string
{
    return match(true) {
        $qualityScore >= 0.8 => 'auto_processing_queue',
        $qualityScore >= 0.6 => 'standard_review_queue',
        $qualityScore >= 0.5 => 'detailed_review_queue',
        default => 'manual_review_queue',
    };
}

echo "Document Routing Based on Quality:\n";
echo str_repeat('=', 60) . "\n";

foreach ($qualityResults as $doc => $data) {
    $queue = routeDocumentByQuality($doc, $data['score']);
    $reprocess = needsReprocessing($data['score'], $data['content_length']);

    echo basename($doc) . ":\n";
    echo "  Route to: $queue\n";

    if ($reprocess) {
        echo "  Action: Reprocess with enhanced settings\n";
    } else {
        echo "  Action: Continue standard workflow\n";
    }

    echo "\n";
}
```
