```php title="PHP"
<?php
declare(strict_types=1);

use GuzzleHttp\Client;

$client = new Client();
$filePath = 'document.pdf';
$fileContent = file_get_contents($filePath);

try {
    $response = $client->post('http://localhost:8000/extract', [
        'multipart' => [
            [
                'name' => 'file',
                'contents' => $fileContent,
                'filename' => basename($filePath),
                'headers' => ['Content-Type' => 'application/pdf'],
            ],
        ],
    ]);

    $result = json_decode((string)$response->getBody(), true);
    echo $result['content'] ?? '';
} catch (Exception $e) {
    echo "Request failed: " . $e->getMessage() . "\n";
}
```
