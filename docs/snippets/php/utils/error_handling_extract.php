```php
<?php

declare(strict_types=1);

/**
 * Error Handling for HTTP/API Extraction
 *
 * Demonstrate error handling when using Kreuzberg extraction via HTTP API.
 * Shows how to properly handle HTTP errors and API response errors.
 */

require_once __DIR__ . '/vendor/autoload.php';

use GuzzleHttp\Client;
use GuzzleHttp\Exception\RequestException;
use GuzzleHttp\Exception\ClientException;
use GuzzleHttp\Exception\ServerException;

/**
 * Extract document via HTTP API with error handling
 *
 * @param string $filePath Path to the document file
 * @param string $apiUrl API endpoint URL
 * @return array|null Extraction results or null on error
 */
function extractViaApi(string $filePath, string $apiUrl = 'http://localhost:8000/extract'): ?array
{
    $client = new Client([
        'timeout' => 30.0,
        'connect_timeout' => 5.0,
    ]);

    try {
        if (!file_exists($filePath)) {
            throw new \RuntimeException("File not found: $filePath");
        }

        $response = $client->post($apiUrl, [
            'multipart' => [
                [
                    'name' => 'files',
                    'contents' => fopen($filePath, 'r'),
                    'filename' => basename($filePath),
                ],
            ],
        ]);

        $results = json_decode($response->getBody()->getContents(), true);

        if (json_last_error() !== JSON_ERROR_NONE) {
            throw new \RuntimeException('Invalid JSON response: ' . json_last_error_msg());
        }

        echo "Success: Extracted " . count($results) . " documents\n";
        return $results;

    } catch (ClientException $e) {
        $response = $e->getResponse();
        $statusCode = $response->getStatusCode();
        $body = json_decode($response->getBody()->getContents(), true);

        $errorType = $body['error_type'] ?? 'Unknown';
        $message = $body['message'] ?? 'No message provided';

        echo "Client Error ($statusCode): $errorType\n";
        echo "Message: $message\n";

        if (isset($body['details'])) {
            echo "Details: " . json_encode($body['details']) . "\n";
        }

        return null;

    } catch (ServerException $e) {
        $response = $e->getResponse();
        $statusCode = $response->getStatusCode();

        echo "Server Error ($statusCode): " . $e->getMessage() . "\n";
        echo "The API server encountered an error. Please try again later.\n";

        return null;

    } catch (RequestException $e) {
        echo "Request Error: " . $e->getMessage() . "\n";

        if ($e->hasResponse()) {
            echo "Response code: " . $e->getResponse()->getStatusCode() . "\n";
        } else {
            echo "No response received - check if the API server is running\n";
        }

        return null;

    } catch (\RuntimeException $e) {
        echo "Runtime Error: " . $e->getMessage() . "\n";
        return null;
    }
}

echo "Attempting to extract document via API...\n";
echo str_repeat('=', 60) . "\n";

$result = extractViaApi('document.pdf');

if ($result !== null) {
    foreach ($result as $doc) {
        $contentLength = strlen($doc['content'] ?? '');
        $mimeType = $doc['mime_type'] ?? 'unknown';

        echo "\nDocument extracted:\n";
        echo "  Content length: $contentLength characters\n";
        echo "  MIME type: $mimeType\n";

        if (isset($doc['metadata'])) {
            echo "  Metadata keys: " . implode(', ', array_keys($doc['metadata'])) . "\n";
        }
    }
} else {
    echo "\nExtraction failed. Check the error messages above.\n";
}

function extractWithRetry(
    string $filePath,
    string $apiUrl = 'http://localhost:8000/extract',
    int $maxRetries = 3,
    float $initialDelay = 1.0
): ?array {
    $attempt = 0;
    $delay = $initialDelay;

    while ($attempt < $maxRetries) {
        $result = extractViaApi($filePath, $apiUrl);

        if ($result !== null) {
            return $result;
        }

        $attempt++;
        if ($attempt < $maxRetries) {
            echo "\nRetrying in " . number_format($delay, 1) . " seconds... (Attempt " . ($attempt + 1) . "/$maxRetries)\n";
            usleep((int)($delay * 1000000));
            $delay *= 2; 
        }
    }

    echo "\nFailed after $maxRetries attempts\n";
    return null;
}

echo "\n" . str_repeat('=', 60) . "\n";
echo "Extracting with retry logic...\n";
echo str_repeat('=', 60) . "\n";

$resultWithRetry = extractWithRetry('document.pdf', 'http://localhost:8000/extract');

if ($resultWithRetry !== null) {
    echo "\nSuccessfully extracted with retry mechanism\n";
}
```
