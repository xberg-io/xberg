```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;

// List all registered OCR backends
$ocr_backends = Kreuzberg::listOcrBackends();
echo "Registered OCR backends:\n";
foreach ($ocr_backends as $backend) {
    echo "  - $backend\n";
}

// List all registered post-processors
$processors = Kreuzberg::listPostProcessors();
echo "Registered post-processors:\n";
foreach ($processors as $processor) {
    echo "  - $processor\n";
}

// List all registered validators
$validators = Kreuzberg::listValidators();
echo "Registered validators:\n";
foreach ($validators as $validator) {
    echo "  - $validator\n";
}

// List all registered document extractors
$extractors = Kreuzberg::listDocumentExtractors();
echo "Registered document extractors:\n";
foreach ($extractors as $extractor) {
    echo "  - $extractor\n";
}
```
