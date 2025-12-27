```php
<?php

declare(strict_types=1);

/**
 * PdfConfig - PDF-Specific Configuration
 *
 * Configure PDF extraction behavior including image quality, text extraction
 * methods, and performance optimization.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\PdfConfig;

$config = new ExtractionConfig(
    pdf: new PdfConfig(
        extractImages: true,
        imageQuality: 85,
        preserveImageFormat: true
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('document.pdf');

echo "PDF extraction complete\n";
echo "Images extracted: " . count($result->images ?? []) . "\n\n";

$highQualityConfig = new ExtractionConfig(
    pdf: new PdfConfig(
        extractImages: true,
        imageQuality: 100,  
        preserveImageFormat: true
    ),
    extractImages: true
);

$kreuzberg = new Kreuzberg($highQualityConfig);
$result = $kreuzberg->extractFile('presentation.pdf');

foreach ($result->images ?? [] as $image) {
    $filename = sprintf('image_%d_page_%d.%s',
        $image->imageIndex,
        $image->pageNumber,
        $image->format
    );
    file_put_contents($filename, $image->data);
    echo "Saved high-quality image: $filename ({$image->width}x{$image->height})\n";
}

$fastConfig = new ExtractionConfig(
    pdf: new PdfConfig(
        extractImages: false,  
        imageQuality: 50       
    ),
    extractTables: false  
);

$kreuzberg = new Kreuzberg($fastConfig);
$start = microtime(true);
$result = $kreuzberg->extractFile('large_document.pdf');
$elapsed = microtime(true) - $start;

echo "\nFast extraction completed in " . number_format($elapsed, 3) . " seconds\n";
echo "Content length: " . strlen($result->content) . " characters\n";
```
