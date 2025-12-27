```php
<?php

declare(strict_types=1);

/**
 * PowerPoint Presentation Extraction
 *
 * This example demonstrates extracting content from PowerPoint files (.pptx, .ppt),
 * including text, notes, images, and tables from slides.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Config\PageConfig;

echo "Example 1: Basic PowerPoint Extraction\n";
echo "======================================\n";

$kreuzberg = new Kreuzberg();
$result = $kreuzberg->extractFile('presentation.pptx');

echo "Content:\n";
echo $result->content . "\n\n";

echo "Metadata:\n";
echo "- Title: " . ($result->metadata->title ?? 'N/A') . "\n";
echo "- Author: " . (isset($result->metadata->authors) ? implode(', ', $result->metadata->authors) : 'N/A') . "\n";
echo "- Slide Count: " . ($result->metadata->pageCount ?? 'N/A') . "\n\n";

echo "Example 2: Extract Per-Slide Content\n";
echo "====================================\n";

$config2 = new ExtractionConfig(
    page: new PageConfig(
        extractPages: true,           
        insertPageMarkers: true,      
        markerFormat: '--- Slide {page_number} ---'
    )
);

$result2 = (new Kreuzberg($config2))->extractFile('presentation.pptx');

if ($result2->pages !== null) {
    echo "Total slides: " . count($result2->pages) . "\n\n";

    foreach ($result2->pages as $page) {
        echo "Slide {$page->pageNumber}:\n";
        echo "- Text length: " . strlen($page->content) . " characters\n";
        echo "- Tables: " . count($page->tables) . "\n";
        echo "- Images: " . count($page->images) . "\n";
        echo "- Content preview: " . substr($page->content, 0, 100) . "...\n\n";
    }
}

echo "Example 3: Extract Images from Slides\n";
echo "=====================================\n";

$config3 = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        minWidth: 100,
        minHeight: 100
    )
);

$result3 = (new Kreuzberg($config3))->extractFile('presentation.pptx');

if ($result3->images !== null) {
    echo "Total images: " . count($result3->images) . "\n\n";

    foreach ($result3->images as $i => $image) {
        echo "Image {$i}:\n";
        echo "- Format: {$image->format}\n";
        echo "- Size: {$image->width}x{$image->height}\n";
        echo "- Slide: {$image->pageNumber}\n";

        $filename = "slide_{$image->pageNumber}_image_{$i}.{$image->format}";
        file_put_contents($filename, base64_decode($image->data));
        echo "- Saved: {$filename}\n\n";
    }
}

echo "Example 4: Extract Tables from Slides\n";
echo "=====================================\n";

$config4 = new ExtractionConfig(
    extractTables: true
);

$result4 = (new Kreuzberg($config4))->extractFile('data_presentation.pptx');

if (count($result4->tables) > 0) {
    echo "Found " . count($result4->tables) . " table(s)\n\n";

    foreach ($result4->tables as $i => $table) {
        echo "Table " . ($i + 1) . " (Slide {$table->pageNumber}):\n";
        echo $table->markdown . "\n\n";
    }
}

echo "Example 5: Convert PowerPoint to Markdown\n";
echo "=========================================\n";

$config5 = new ExtractionConfig(
    page: new PageConfig(
        extractPages: true,
        insertPageMarkers: true,
        markerFormat: '---\n\n## Slide {page_number}\n\n'
    ),
    outputFormat: 'markdown'
);

$result5 = (new Kreuzberg($config5))->extractFile('presentation.pptx');

$markdownContent = $result5->content;
file_put_contents('presentation.md', $markdownContent);

echo "Converted to Markdown\n";
echo "Saved as: presentation.md\n";
echo "Content preview:\n";
echo substr($markdownContent, 0, 500) . "...\n\n";

echo "Example 6: Generate Presentation Summary\n";
echo "========================================\n";

$config6 = new ExtractionConfig(
    page: new PageConfig(extractPages: true)
);

$result6 = (new Kreuzberg($config6))->extractFile('meeting_deck.pptx');

echo "Presentation Summary:\n";
echo "====================\n";
echo "Title: " . ($result6->metadata->title ?? 'Untitled') . "\n";
echo "Author: " . (isset($result6->metadata->authors) ? implode(', ', $result6->metadata->authors) : 'Unknown') . "\n";
echo "Total Slides: " . ($result6->metadata->pageCount ?? count($result6->pages ?? [])) . "\n";
echo "Total Text: " . strlen($result6->content) . " characters\n";
echo "Tables: " . count($result6->tables) . "\n";

if ($result6->pages !== null) {
    echo "\nSlide Breakdown:\n";
    foreach ($result6->pages as $page) {
        $wordCount = str_word_count($page->content);
        echo "- Slide {$page->pageNumber}: {$wordCount} words, " . count($page->tables) . " tables\n";
    }
}

echo "\n";

echo "Example 7: Search Content in Slides\n";
echo "===================================\n";

$config7 = new ExtractionConfig(
    page: new PageConfig(extractPages: true)
);

$result7 = (new Kreuzberg($config7))->extractFile('presentation.pptx');

$searchTerm = "revenue";

if ($result7->pages !== null) {
    echo "Searching for '{$searchTerm}':\n\n";

    foreach ($result7->pages as $page) {
        if (stripos($page->content, $searchTerm) !== false) {
            echo "Found in Slide {$page->pageNumber}:\n";

            $pos = stripos($page->content, $searchTerm);
            $context = substr($page->content, max(0, $pos - 50), 150);
            echo "- Context: ...{$context}...\n\n";
        }
    }
}

echo "\nSupported PowerPoint Formats:\n";
echo "=============================\n";
echo "- .pptx (PowerPoint 2007+)\n";
echo "- .ppt (PowerPoint 97-2003)\n";
echo "- .pptm (Macro-enabled)\n";
echo "- .potx (Template)\n";

echo "\n\nBest Practices:\n";
echo "===============\n";
echo "1. Use page extraction to process individual slides\n";
echo "2. Extract images for visual content analysis\n";
echo "3. Extract tables for data analysis\n";
echo "4. Use metadata for presentation information\n";
echo "5. Convert to Markdown for documentation\n";
echo "6. Search across slides for specific content\n";
echo "7. Generate summaries for presentation overviews\n";
```
