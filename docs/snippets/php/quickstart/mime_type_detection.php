```php title="mime_type_detection.php"
<?php

declare(strict_types=1);

/**
 * MIME Type Detection
 *
 * Xberg can automatically detect MIME types from file content or paths.
 * This is useful when the file extension is missing or unreliable.
 */

require_once __DIR__ . '/vendor/autoload.php';

use function Xberg\detect_mime_type;
use function Xberg\detect_mime_type_from_path;
use function Xberg\extract;

$path = 'document.pdf';
$mimeType = detect_mime_type_from_path($path);
echo "Detected MIME type from path: $mimeType\n";

$data = file_get_contents($path);
$mimeType = detect_mime_type($data);
echo "Detected MIME type from content: $mimeType\n\n";

$unknownFile = 'file_without_extension';
if (file_exists($unknownFile)) {
    $detectedType = detect_mime_type_from_path($unknownFile);
    echo "Unknown file detected as: $detectedType\n";

    $result = extract($unknownFile, $detectedType);
    echo "Successfully extracted " . strlen($result->content) . " characters\n";
}

$allowedTypes = [
    'application/pdf',
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    'text/plain',
];

$fileToCheck = 'user_upload.dat';
if (file_exists($fileToCheck)) {
    $type = detect_mime_type_from_path($fileToCheck);

    if (in_array($type, $allowedTypes, true)) {
        echo "File type $type is allowed, processing...\n";
        $result = extract($fileToCheck);
    } else {
        echo "File type $type is not allowed\n";
    }
}
```
