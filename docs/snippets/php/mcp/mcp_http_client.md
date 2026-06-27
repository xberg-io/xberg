```php title="PHP"
<?php

declare(strict_types=1);

use GuzzleHttp\Client;
use GuzzleHttp\Exception\GuzzleException;

/**
 * Connect to Xberg MCP server via HTTP transport.
 *
 * Requires MCP server running with HTTP transport:
 * xberg mcp --transport http --port 3000
 */
final readonly class XbergMcpClient
{
    private Client $http;

    public function __construct(
        private string $baseUrl = 'http://localhost:3000',
    ) {
        $this->http = new Client([
            'base_uri' => $this->baseUrl,
            'timeout' => 30.0,
            'headers' => [
                'Content-Type' => 'application/json',
                'Accept' => 'application/json',
            ],
        ]);
    }

    /**
     * @return array<string, mixed>
     * @throws GuzzleException
     */
    public function initialize(): array
    {
        $response = $this->http->post('/initialize', [
            'json' => [
                'protocolVersion' => '2024-11-05',
                'capabilities' => [],
                'clientInfo' => [
                    'name' => 'xberg-php-client',
                    'version' => '4.2.7',
                ],
            ],
        ]);

        return json_decode($response->getBody()->getContents(), true, 512, JSON_THROW_ON_ERROR);
    }

    /**
     * @return array<int, array{name: string, description: string, inputSchema: array<string, mixed>}>
     * @throws GuzzleException
     */
    public function listTools(): array
    {
        $response = $this->http->post('/tools/list');
        $data = json_decode($response->getBody()->getContents(), true, 512, JSON_THROW_ON_ERROR);

        return $data['tools'] ?? [];
    }

    /**
     * @param array<string, mixed> $arguments
     * @return array<string, mixed>
     * @throws GuzzleException
     */
    public function callTool(string $toolName, array $arguments): array
    {
        $response = $this->http->post('/tools/call', [
            'json' => [
                'name' => $toolName,
                'arguments' => $arguments,
            ],
        ]);

        return json_decode($response->getBody()->getContents(), true, 512, JSON_THROW_ON_ERROR);
    }

    /**
     * @param array<string, mixed>|null $config
     * @return array<string, mixed>
     * @throws GuzzleException
     */
    public function extract(string $path, ?array $config = null): array
    {
        return $this->callTool('extract', [
            'path' => $path,
            'config' => $config,
        ]);
    }

    /**
     * @param array<int, string> $paths
     * @param array<string, mixed>|null $config
     * @return array<string, mixed>
     * @throws GuzzleException
     */
    public function extractBatch(array $paths, ?array $config = null): array
    {
        return $this->callTool('extract_batch', [
            'paths' => $paths,
            'config' => $config,
        ]);
    }
}

// Usage
$client = new XbergMcpClient('http://localhost:3000');

// Initialize connection
$serverInfo = $client->initialize();
echo "Connected to: {$serverInfo['serverInfo']['name']}\n";

// List available tools
$tools = $client->listTools();
$toolNames = array_column($tools, 'name');
echo "Available tools: " . implode(', ', $toolNames) . "\n";

// Extract a file
$result = $client->extract('document.pdf');
echo "Extracted content length: " . strlen($result['content']) . "\n";

// Batch extract
$results = $client->extractBatch([
    'file1.pdf',
    'file2.docx',
    'file3.md',
]);
echo "Batch extracted " . count($results) . " files\n";
```
