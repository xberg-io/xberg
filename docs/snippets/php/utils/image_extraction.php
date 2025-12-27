```php
<?php

declare(strict_types=1);

/**
 * Image Extraction from Documents
 *
 * Extract embedded images from PDF and other document formats.
 * Demonstrates saving images, analyzing metadata, and processing image data.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\PdfConfig;
use Kreuzberg\Result\ExtractedImage;

$config = new ExtractionConfig(
    extractImages: true,
    pdf: new PdfConfig(
        extractImages: true,
        imageQuality: 90
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('document_with_images.pdf');

echo "Image Extraction Results:\n";
echo str_repeat('=', 60) . "\n";
echo "Total images extracted: " . count($result->images ?? []) . "\n\n";

$outputDir = './extracted_images';
if (!is_dir($outputDir)) {
    mkdir($outputDir, 0755, true);
}

foreach ($result->images ?? [] as $index => $image) {
    echo "Image " . ($index + 1) . ":\n";
    echo str_repeat('-', 40) . "\n";

    $filename = sprintf(
        'page_%d_image_%d.%s',
        $image->pageNumber ?? 0,
        $image->imageIndex ?? $index,
        $image->format ?? 'png'
    );
    $filepath = $outputDir . '/' . $filename;

    $bytesWritten = file_put_contents($filepath, $image->data);

    if ($bytesWritten !== false) {
        echo "  Saved: $filename\n";
        echo "  Size: {$image->width}x{$image->height} pixels\n";
        echo "  Format: {$image->format}\n";
        echo "  File size: " . number_format($bytesWritten) . " bytes\n";
        echo "  Page: " . ($image->pageNumber ?? 'N/A') . "\n";

        if ($image->width > 0 && $image->height > 0) {
            $aspectRatio = $image->width / $image->height;
            echo "  Aspect ratio: " . number_format($aspectRatio, 2) . ":1\n";

            $orientation = $image->width > $image->height ? 'Landscape' : 'Portrait';
            if (abs($image->width - $image->height) < 10) {
                $orientation = 'Square';
            }
            echo "  Orientation: $orientation\n";
        }

        echo "\n";
    } else {
        echo "  Error: Failed to save image\n\n";
    }
}

echo "Image Analysis:\n";
echo str_repeat('=', 60) . "\n";

if (!empty($result->images)) {
    $largeImages = array_filter(
        $result->images,
        fn(ExtractedImage $img) => $img->width > 800 || $img->height > 800
    );

    echo "Large images (>800px): " . count($largeImages) . "\n";

    $totalBytes = array_sum(
        array_map(fn(ExtractedImage $img) => strlen($img->data), $result->images)
    );

    echo "Total image data: " . number_format($totalBytes / 1024, 2) . " KB\n";

    $formatCounts = [];
    foreach ($result->images as $image) {
        $format = $image->format ?? 'unknown';
        $formatCounts[$format] = ($formatCounts[$format] ?? 0) + 1;
    }

    echo "\nImages by format:\n";
    foreach ($formatCounts as $format => $count) {
        echo "  $format: $count\n";
    }

    $totalWidth = array_sum(array_map(fn($img) => $img->width, $result->images));
    $totalHeight = array_sum(array_map(fn($img) => $img->height, $result->images));
    $imageCount = count($result->images);

    echo "\nAverage dimensions: " .
        round($totalWidth / $imageCount) . "x" .
        round($totalHeight / $imageCount) . " pixels\n";
}

function createThumbnail(ExtractedImage $image, int $maxWidth = 200): ?string
{
    if ($image->width <= $maxWidth) {
        return null; 
    }

    $scale = $maxWidth / $image->width;
    $newHeight = (int)($image->height * $scale);

    return "Thumbnail would be: {$maxWidth}x{$newHeight}";
}

echo "\nThumbnail recommendations:\n";
foreach ($result->images ?? [] as $index => $image) {
    $thumbInfo = createThumbnail($image, 200);
    if ($thumbInfo !== null) {
        echo "  Image " . ($index + 1) . ": $thumbInfo\n";
    }
}
```
