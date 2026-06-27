```php title="PHP"
<?php

declare(strict_types=1);

namespace App\Services;

use GuzzleHttp\Client;
use GuzzleHttp\Exception\GuzzleException;
use Illuminate\Support\Facades\Cache;
use Illuminate\Support\Facades\Log;
use Psr\Log\LoggerInterface;

/**
 * Laravel service for Xberg MCP integration.
 *
 * Register in AppServiceProvider:
 * $this->app->singleton(XbergMcpService::class);
 *
 * Then inject in controllers or jobs.
 */
final class XbergMcpService
{
    private Client $http;

    public function __construct(
        private readonly LoggerInterface $logger,
        private readonly string $mcpUrl = 'http://localhost:3000',
    ) {
        $this->http = new Client([
            'base_uri' => $this->mcpUrl,
            'timeout' => config('xberg.timeout', 30),
            'headers' => [
                'Content-Type' => 'application/json',
                'Accept' => 'application/json',
            ],
        ]);
    }

    /**
     * Extract content from a file via MCP server.
     *
     * @param array<string, mixed>|null $config
     * @return array{content: string, mimeType: string, metadata: array<string, mixed>}
     * @throws GuzzleException
     */
    public function extract(string $path, ?array $config = null): array
    {
        $cacheKey = 'xberg_extract_' . md5($path . json_encode($config));

        return Cache::remember($cacheKey, now()->addHours(24), function () use ($path, $config): array {
            $this->logger->info('Extracting file via MCP', [
                'path' => $path,
                'config' => $config,
            ]);

            try {
                $response = $this->http->post('/tools/call', [
                    'json' => [
                        'name' => 'extract',
                        'arguments' => [
                            'path' => $path,
                            'config' => $config,
                        ],
                    ],
                ]);

                $result = json_decode(
                    $response->getBody()->getContents(),
                    true,
                    512,
                    JSON_THROW_ON_ERROR
                );

                $this->logger->info('Extraction successful', [
                    'path' => $path,
                    'content_length' => strlen($result['content'] ?? ''),
                ]);

                return $result;
            } catch (GuzzleException $e) {
                $this->logger->error('MCP extraction failed', [
                    'path' => $path,
                    'error' => $e->getMessage(),
                ]);

                throw $e;
            }
        });
    }

    /**
     * Extract multiple files in batch.
     *
     * @param array<int, string> $paths
     * @param array<string, mixed>|null $config
     * @return array<int, array{content: string, mimeType: string}>
     * @throws GuzzleException
     */
    public function extractBatch(array $paths, ?array $config = null): array
    {
        $this->logger->info('Batch extracting files via MCP', [
            'count' => count($paths),
        ]);

        try {
            $response = $this->http->post('/tools/call', [
                'json' => [
                    'name' => 'extract_batch',
                    'arguments' => [
                        'paths' => $paths,
                        'config' => $config,
                    ],
                ],
            ]);

            $results = json_decode(
                $response->getBody()->getContents(),
                true,
                512,
                JSON_THROW_ON_ERROR
            );

            $this->logger->info('Batch extraction successful', [
                'count' => count($results),
            ]);

            return $results;
        } catch (GuzzleException $e) {
            $this->logger->error('MCP batch extraction failed', [
                'count' => count($paths),
                'error' => $e->getMessage(),
            ]);

            throw $e;
        }
    }

    /**
     * Check if MCP server is healthy.
     */
    public function healthCheck(): bool
    {
        try {
            $response = $this->http->get('/health', ['timeout' => 2]);

            return $response->getStatusCode() === 200;
        } catch (GuzzleException) {
            return false;
        }
    }
}

// Usage in a Laravel controller
namespace App\Http\Controllers;

use App\Services\XbergMcpService;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;

final class DocumentController extends Controller
{
    public function __construct(
        private readonly XbergMcpService $xberg,
    ) {}

    public function extract(Request $request): JsonResponse
    {
        $validated = $request->validate([
            'file_path' => 'required|string',
            'extract_tables' => 'boolean',
            'extract_images' => 'boolean',
        ]);

        $config = [
            'extractTables' => $validated['extract_tables'] ?? true,
            'extractImages' => $validated['extract_images'] ?? false,
        ];

        $result = $this->xberg->extract(
            $validated['file_path'],
            $config
        );

        return response()->json([
            'success' => true,
            'data' => $result,
        ]);
    }

    public function batchExtract(Request $request): JsonResponse
    {
        $validated = $request->validate([
            'file_paths' => 'required|array',
            'file_paths.*' => 'required|string',
        ]);

        $results = $this->xberg->extractBatch($validated['file_paths']);

        return response()->json([
            'success' => true,
            'data' => $results,
            'count' => count($results),
        ]);
    }

    public function health(): JsonResponse
    {
        $healthy = $this->xberg->healthCheck();

        return response()->json([
            'healthy' => $healthy,
            'service' => 'xberg-mcp',
        ], $healthy ? 200 : 503);
    }
}
```
