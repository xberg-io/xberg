```php title="PHP"
<?php
declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\ImageExtractionConfig;

// Extract images from documents alongside text
$config = new ExtractionConfig(
    images: new ImageExtractionConfig(
        extractImages: true,
        embedAsBase64: false,  // Save images to disk
        maxImagesPerPage: 10
    )
);

$xberg = new Xberg($config);
$result = $xberg->extract('document_with_images.pdf');

echo "Extracted Content:\n";
echo $result->content . "\n\n";

if (!empty($result->images)) {
    echo "Extracted " . count($result->images) . " images\n";
    foreach ($result->images as $index => $image) {
        echo "Image " . ($index + 1) . ":\n";
        echo "  Type: " . $image->mimeType . "\n";
        echo "  Size: " . strlen($image->data) . " bytes\n";
        if (isset($image->width) && isset($image->height)) {
            echo "  Dimensions: " . $image->width . "x" . $image->height . "\n";
        }
        echo "\n";
    }
}
?>
```
