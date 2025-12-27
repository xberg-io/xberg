```php
<?php

declare(strict_types=1);

/**
 * Multilingual Document Language Detection
 *
 * Detect multiple languages in documents that contain mixed-language content.
 * Useful for processing multilingual documents, translations, and international content.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\LanguageDetectionConfig;

$config = new ExtractionConfig(
    languageDetection: new LanguageDetectionConfig(
        enabled: true,
        minConfidence: 0.7,
        detectMultiple: true  
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('multilingual_document.pdf');

echo "Multilingual Language Detection:\n";
echo str_repeat('=', 60) . "\n";
echo "Document: multilingual_document.pdf\n\n";

$detectedLanguages = $result->detectedLanguages ?? [];
$languageCount = count($detectedLanguages);

echo "Detected $languageCount language(s): " . implode(', ', $detectedLanguages) . "\n\n";

if ($languageCount > 1) {
    echo "This is a multilingual document.\n";
    echo "Languages present:\n";

    foreach ($detectedLanguages as $index => $language) {
        $label = $index === 0 ? 'Primary' : 'Secondary';
        echo "  $label: $language\n";
    }

    echo "\n";
} elseif ($languageCount === 1) {
    echo "This is a monolingual document.\n";
    echo "Language: {$detectedLanguages[0]}\n\n";
} else {
    echo "No languages detected.\n\n";
}

if (isset($result->metadata['language_distribution'])) {
    echo "Language Distribution:\n";
    echo str_repeat('-', 40) . "\n";

    foreach ($result->metadata['language_distribution'] as $lang => $percentage) {
        $barLength = (int)($percentage * 40);
        $bar = str_repeat('█', $barLength);

        echo sprintf(
            "  %-10s [%-40s] %5.1f%%\n",
            $lang,
            $bar,
            $percentage * 100
        );
    }

    echo "\n";
}

function categorizeMultilingualDocument(array $languages): string
{
    $count = count($languages);

    if ($count === 0) {
        return 'unknown';
    }

    if ($count === 1) {
        return 'monolingual';
    }

    if ($count === 2) {
        sort($languages);
        $pair = implode('-', $languages);

        $commonPairs = [
            'en-es' => 'English-Spanish bilingual',
            'en-fr' => 'English-French bilingual',
            'en-de' => 'English-German bilingual',
            'en-zh' => 'English-Chinese bilingual',
        ];

        return $commonPairs[$pair] ?? 'bilingual';
    }

    return 'multilingual';
}

$docType = categorizeMultilingualDocument($detectedLanguages);
echo "Document type: $docType\n\n";

if ($languageCount > 1) {
    echo "Multilingual Processing Recommendations:\n";
    echo str_repeat('=', 60) . "\n";

    echo "1. Consider splitting content by language\n";
    echo "2. Use language-specific OCR models if available\n";
    echo "3. Apply appropriate tokenization for each language\n";
    echo "4. Use multilingual embedding models for semantic search\n\n";
}

function extractLanguageSections(string $content, array $languages): array
{

    $sections = [];
    $lines = explode("\n", $content);
    $currentLang = $languages[0] ?? 'unknown';

    foreach ($lines as $line) {
        if (empty(trim($line))) {
            continue;
        }

        if (!isset($sections[$currentLang])) {
            $sections[$currentLang] = [];
        }

        $sections[$currentLang][] = $line;
    }

    return $sections;
}

$testDocuments = [
    'english_only.pdf',
    'spanish_english.pdf',
    'multilingual_eu.pdf',
];

echo "Batch Multilingual Analysis:\n";
echo str_repeat('=', 60) . "\n";

$multilingualConfig = new ExtractionConfig(
    languageDetection: new LanguageDetectionConfig(
        enabled: true,
        minConfidence: 0.6,
        detectMultiple: true
    )
);

$kreuzberg = new Kreuzberg($multilingualConfig);

$statistics = [
    'monolingual' => 0,
    'bilingual' => 0,
    'multilingual' => 0,
];

foreach ($testDocuments as $document) {
    if (!file_exists($document)) {
        echo basename($document) . ": File not found\n";
        continue;
    }

    $result = $kreuzberg->extractFile($document);
    $languages = $result->detectedLanguages ?? [];
    $type = categorizeMultilingualDocument($languages);

    echo basename($document) . ":\n";
    echo "  Languages: " . implode(', ', $languages) . "\n";
    echo "  Type: $type\n\n";

    if (count($languages) === 1) {
        $statistics['monolingual']++;
    } elseif (count($languages) === 2) {
        $statistics['bilingual']++;
    } elseif (count($languages) > 2) {
        $statistics['multilingual']++;
    }
}

echo "Statistics:\n";
echo "  Monolingual: {$statistics['monolingual']}\n";
echo "  Bilingual: {$statistics['bilingual']}\n";
echo "  Multilingual: {$statistics['multilingual']}\n\n";

function analyzeLanguagePairs(array $documents, Kreuzberg $kreuzberg): array
{
    $pairs = [];

    foreach ($documents as $document) {
        if (!file_exists($document)) {
            continue;
        }

        $result = $kreuzberg->extractFile($document);
        $languages = $result->detectedLanguages ?? [];

        if (count($languages) >= 2) {
            sort($languages);
            $pair = implode('-', array_slice($languages, 0, 2));

            if (!isset($pairs[$pair])) {
                $pairs[$pair] = 0;
            }

            $pairs[$pair]++;
        }
    }

    arsort($pairs);
    return $pairs;
}

$translationPairs = [
    'en-es' => 'English ↔ Spanish',
    'en-fr' => 'English ↔ French',
    'en-de' => 'English ↔ German',
    'en-zh' => 'English ↔ Chinese',
    'en-ja' => 'English ↔ Japanese',
];

echo "Common Translation Pairs:\n";
echo str_repeat('=', 60) . "\n";

foreach ($translationPairs as $code => $name) {
    echo "  $code: $name\n";
}

echo "\nUse these configurations for translation document processing.\n";
```
