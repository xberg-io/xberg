```php title="PHP"
<?php declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;

// Extract PDF metadata
$result = Xberg::extract_sync("document.pdf", null, new ExtractionConfig());

if ($result->metadata?->pdf) {
    $pdfMeta = $result->metadata->pdf;
    if ($pdfMeta->page_count !== null) {
        echo "Pages: " . $pdfMeta->page_count . "\n";
    }
    if ($pdfMeta->author !== null) {
        echo "Author: " . $pdfMeta->author . "\n";
    }
    if ($pdfMeta->title !== null) {
        echo "Title: " . $pdfMeta->title . "\n";
    }
}

// Extract HTML metadata
$htmlResult = Xberg::extract_sync("page.html", null, new ExtractionConfig());

if ($htmlResult->metadata?->html) {
    $htmlMeta = $htmlResult->metadata->html;
    if ($htmlMeta->title !== null) {
        echo "Title: " . $htmlMeta->title . "\n";
    }
    if ($htmlMeta->description !== null) {
        echo "Description: " . $htmlMeta->description . "\n";
    }

    // Access keywords array
    echo "Keywords: " . implode(", ", $htmlMeta->keywords ?? []) . "\n";

    // Access canonical URL
    if ($htmlMeta->canonical_url !== null) {
        echo "Canonical: " . $htmlMeta->canonical_url . "\n";
    }

    // Access Open Graph fields
    if (!empty($htmlMeta->open_graph)) {
        if (isset($htmlMeta->open_graph["image"])) {
            echo "OG Image: " . $htmlMeta->open_graph["image"] . "\n";
        }
    }

    // Access language
    if ($htmlMeta->language !== null) {
        echo "Language: " . $htmlMeta->language . "\n";
    }

    // Access headers
    if (!empty($htmlMeta->headers)) {
        foreach ($htmlMeta->headers as $header) {
            echo "Header (level " . $header->level . "): " . $header->text . "\n";
        }
    }
}
?>
```
