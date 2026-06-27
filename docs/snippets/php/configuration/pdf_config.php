```php title="pdf_config.php"
<?php

declare(strict_types=1);

/**
 * PdfConfig - PDF-Specific Configuration
 *
 * Configure PDF extraction behavior including image quality, text extraction
 * methods, and performance optimization.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\PdfConfig;

$config = new ExtractionConfig(
    pdf: new PdfConfig(
        extractImages: true,
        imageQuality: 85,
        preserveImageFormat: true
    )
);

$xberg = new Xberg($config);
$result = $xberg->extract('document.pdf');

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

$xberg = new Xberg($highQualityConfig);
$result = $xberg->extract('presentation.pdf');

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

$xberg = new Xberg($fastConfig);
$start = microtime(true);
$result = $xberg->extract('large_document.pdf');
$elapsed = microtime(true) - $start;

echo "\nFast extraction completed in " . number_format($elapsed, 3) . " seconds\n";
echo "Content length: " . strlen($result->content) . " characters\n";
```
