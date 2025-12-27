```php
<?php

declare(strict_types=1);

/**
 * Docker Kreuzberg Client
 *
 * This example demonstrates how to interact with Kreuzberg running in a Docker container.
 * It shows how to start a container, extract content from files via the API, and cleanup.
 */

class DockerKreuzbergClient
{
    private string $containerName;
    private string $containerImage;
    private int $apiPort;
    private string $apiUrl;

    public function __construct(
        string $containerName = 'kreuzberg-api',
        string $containerImage = 'kreuzberg:latest',
        int $apiPort = 8000
    ) {
        $this->containerName = $containerName;
        $this->containerImage = $containerImage;
        $this->apiPort = $apiPort;
        $this->apiUrl = "http://localhost:{$apiPort}/api/extract";
    }

    /**
     * Start the Kreuzberg Docker container
     *
     * @throws RuntimeException if container fails to start
     */
    public function startContainer(): void
    {
        echo "Starting Kreuzberg Docker container...\n";

        $cmd = sprintf(
            'docker run -d --name %s -p %d:8000 %s',
            escapeshellarg($this->containerName),
            $this->apiPort,
            escapeshellarg($this->containerImage)
        );

        exec($cmd, $output, $returnCode);

        if ($returnCode !== 0) {
            throw new RuntimeException("Failed to start container: " . implode("\n", $output));
        }

        echo "Container started on http://localhost:{$this->apiPort}\n";
    }

    /**
     * Extract content from a file using the Docker API
     *
     * @param string $filePath Path to the file to extract
     * @return string Extracted content
     * @throws RuntimeException if extraction fails
     */
    public function extractFile(string $filePath): string
    {
        if (!file_exists($filePath)) {
            throw new RuntimeException("File not found: {$filePath}");
        }

        $boundary = '----WebKitFormBoundary' . bin2hex(random_bytes(16));
        $fileContent = file_get_contents($filePath);
        $fileName = basename($filePath);

        $body = "--{$boundary}\r\n";
        $body .= "Content-Disposition: form-data; name=\"file\"; filename=\"{$fileName}\"\r\n";
        $body .= "Content-Type: application/octet-stream\r\n\r\n";
        $body .= $fileContent;
        $body .= "\r\n--{$boundary}--\r\n";

        $ch = curl_init($this->apiUrl);
        curl_setopt_array($ch, [
            CURLOPT_POST => true,
            CURLOPT_POSTFIELDS => $body,
            CURLOPT_HTTPHEADER => [
                "Content-Type: multipart/form-data; boundary={$boundary}",
                "Content-Length: " . strlen($body),
            ],
            CURLOPT_RETURNTRANSFER => true,
        ]);

        $response = curl_exec($ch);
        $httpCode = curl_getinfo($ch, CURLINFO_HTTP_CODE);

        if ($response === false) {
            $error = curl_error($ch);
            curl_close($ch);
            throw new RuntimeException("cURL error: {$error}");
        }

        curl_close($ch);

        if ($httpCode !== 200) {
            throw new RuntimeException("HTTP error {$httpCode}: {$response}");
        }

        $result = json_decode($response, true);
        if (json_last_error() !== JSON_ERROR_NONE) {
            throw new RuntimeException("JSON decode error: " . json_last_error_msg());
        }

        return $result['content'] ?? '';
    }

    /**
     * Stop and remove the Docker container
     */
    public function stopContainer(): void
    {
        echo "Stopping Kreuzberg Docker container...\n";

        exec(sprintf('docker stop %s', escapeshellarg($this->containerName)), $output, $returnCode);
        exec(sprintf('docker rm %s', escapeshellarg($this->containerName)), $output, $returnCode);

        echo "Container stopped and removed\n";
    }
}

$dockerClient = new DockerKreuzbergClient();

try {
    $dockerClient->startContainer();

    sleep(2);

    $content = $dockerClient->extractFile('document.pdf');
    echo "Extracted content:\n{$content}\n";
} finally {
    $dockerClient->stopContainer();
}
```
