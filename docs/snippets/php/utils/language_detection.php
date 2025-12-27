```php
<?php

declare(strict_types=1);

/**
 * Language Detection
 *
 * Automatically detect the language of extracted document content.
 * Useful for routing documents to language-specific processing pipelines.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\LanguageDetectionConfig;

$config = new ExtractionConfig(
    languageDetection: new LanguageDetectionConfig(
        enabled: true,
        minConfidence: 0.9,
        detectMultiple: true
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('document.pdf');

echo "Language Detection Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Document: document.pdf\n";
echo "Content length: " . strlen($result->content) . " characters\n\n";

$detectedLanguages = $result->detectedLanguages ?? [];

if (!empty($detectedLanguages)) {
    echo "Detected languages: " . implode(', ', $detectedLanguages) . "\n";

    $primaryLanguage = $detectedLanguages[0];
    echo "Primary language: $primaryLanguage\n\n";

    if (isset($result->metadata['language_confidence'])) {
        echo "Language confidence scores:\n";
        foreach ($result->metadata['language_confidence'] as $lang => $confidence) {
            echo sprintf("  %-10s: %.1f%%\n", $lang, $confidence * 100);
        }
        echo "\n";
    }
} else {
    echo "No language detected or confidence too low.\n";
    echo "Try lowering minConfidence threshold.\n\n";
}

if (!empty($detectedLanguages)) {
    $primaryLanguage = $detectedLanguages[0];

    match ($primaryLanguage) {
        'en', 'eng' => echo "Processing as English document...\n",
        'es', 'spa' => echo "Processing as Spanish document...\n",
        'fr', 'fra' => echo "Processing as French document...\n",
        'de', 'deu' => echo "Processing as German document...\n",
        'zh', 'zho' => echo "Processing as Chinese document...\n",
        default => echo "Processing as $primaryLanguage document...\n",
    };
}

echo "\n" . str_repeat('=', 60) . "\n";
echo "Testing Different Confidence Thresholds:\n";
echo str_repeat('=', 60) . "\n";

$thresholds = [0.5, 0.7, 0.9, 0.95];

foreach ($thresholds as $threshold) {
    $thresholdConfig = new ExtractionConfig(
        languageDetection: new LanguageDetectionConfig(
            enabled: true,
            minConfidence: $threshold,
            detectMultiple: true
        )
    );

    $kreuzberg = new Kreuzberg($thresholdConfig);
    $result = $kreuzberg->extractFile('document.pdf');

    $languages = $result->detectedLanguages ?? [];

    echo sprintf("Threshold %.2f: ", $threshold);
    if (!empty($languages)) {
        echo implode(', ', $languages) . "\n";
    } else {
        echo "No languages detected\n";
    }
}

function getLanguageName(string $code): string
{
    $languageNames = [
        'en' => 'English',
        'es' => 'Spanish',
        'fr' => 'French',
        'de' => 'German',
        'it' => 'Italian',
        'pt' => 'Portuguese',
        'ru' => 'Russian',
        'zh' => 'Chinese',
        'ja' => 'Japanese',
        'ko' => 'Korean',
        'ar' => 'Arabic',
        'hi' => 'Hindi',
        'nl' => 'Dutch',
        'pl' => 'Polish',
        'tr' => 'Turkish',
    ];

    return $languageNames[$code] ?? ucfirst($code);
}

echo "\n" . str_repeat('=', 60) . "\n";
echo "Detected Languages (Full Names):\n";
echo str_repeat('=', 60) . "\n";

if (!empty($detectedLanguages)) {
    foreach ($detectedLanguages as $langCode) {
        echo "  - " . getLanguageName($langCode) . " ($langCode)\n";
    }
} else {
    echo "No languages detected.\n";
}

$documents = [
    'english_doc.pdf',
    'spanish_doc.pdf',
    'german_doc.pdf',
];

echo "\n" . str_repeat('=', 60) . "\n";
echo "Batch Language Detection:\n";
echo str_repeat('=', 60) . "\n";

$detectionConfig = new ExtractionConfig(
    languageDetection: new LanguageDetectionConfig(
        enabled: true,
        minConfidence: 0.8,
        detectMultiple: false  
    )
);

$kreuzberg = new Kreuzberg($detectionConfig);

foreach ($documents as $document) {
    if (!file_exists($document)) {
        echo basename($document) . ": File not found\n";
        continue;
    }

    $result = $kreuzberg->extractFile($document);
    $languages = $result->detectedLanguages ?? [];

    echo basename($document) . ": ";

    if (!empty($languages)) {
        $primaryLang = $languages[0];
        echo getLanguageName($primaryLang) . " ($primaryLang)\n";
    } else {
        echo "Language not detected\n";
    }
}

function routeDocumentByLanguage(string $filePath, array $detectedLanguages): string
{
    if (empty($detectedLanguages)) {
        return 'default_queue';
    }

    $primaryLanguage = $detectedLanguages[0];

    return match ($primaryLanguage) {
        'en', 'eng' => 'english_processing_queue',
        'es', 'spa' => 'spanish_processing_queue',
        'fr', 'fra' => 'french_processing_queue',
        'de', 'deu' => 'german_processing_queue',
        'zh', 'zho', 'ja', 'jpn', 'ko', 'kor' => 'cjk_processing_queue',
        'ar', 'ara', 'he', 'heb' => 'rtl_processing_queue',
        default => 'multilingual_queue',
    };
}

echo "\n" . str_repeat('=', 60) . "\n";
echo "Document Routing Based on Language:\n";
echo str_repeat('=', 60) . "\n";

if (!empty($detectedLanguages)) {
    $queue = routeDocumentByLanguage('document.pdf', $detectedLanguages);
    echo "Document routed to: $queue\n";
}
```
