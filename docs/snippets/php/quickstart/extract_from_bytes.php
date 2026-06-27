```php title="extract_from_bytes.php"
<?php

declare(strict_types=1);

/**
 * Extracting from Bytes
 *
 * Extract content from file data in memory instead of from disk.
 * Useful for processing uploaded files or data from remote sources.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;

$fileData = file_get_contents('document.pdf');
$mimeType = 'application/pdf';

$result = extract($fileData, $mimeType);
echo "Extracted using procedural API:\n";
echo substr($result->getContent(), 0, 200) . "...\n\n";

$result = $xberg->extract($fileData, $mimeType);
echo "Extracted using OOP API:\n";
echo substr($result->getContent(), 0, 200) . "...\n\n";

$uploadedFile = [
    'tmp_name' => '/tmp/uploaded_document.pdf',
    'type' => 'application/pdf',
    'size' => 1024000,
];

if (file_exists($uploadedFile['tmp_name'])) {
    $data = file_get_contents($uploadedFile['tmp_name']);
    $result = extract($data, $uploadedFile['type']);

    echo "Uploaded file processed:\n";
    echo "Size: " . strlen($data) . " bytes\n";
    echo "Content length: " . strlen($result->getContent()) . " characters\n";
}
```
