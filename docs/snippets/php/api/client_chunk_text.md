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
            [
                'name' => 'chunking',
                'contents' => json_encode(['max_characters' => 800, 'overlap' => 100]),
            ],
        ],
    ]);

    $result = json_decode((string)$response->getBody(), true);
    if (isset($result['chunks']) && is_array($result['chunks'])) {
        echo count($result['chunks']) . " chunks\n";
        foreach ($result['chunks'] as $chunk) {
            echo "  " . strlen($chunk['content'] ?? '') . " chars\n";
        }
    }
} catch (Exception $e) {
    echo "Request failed: " . $e->getMessage() . "\n";
}
```
