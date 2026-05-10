```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;

class StatefulPlugin implements PostProcessor {
    private int $callCount = 0;
    private array $cache = [];

    public function name(): string {
        return "stateful-plugin";
    }

    public function version(): string {
        return "1.0.0";
    }

    public function initialize(): void {
        $this->callCount = 0;
        $this->cache = [];
        error_log("StatefulPlugin initialized");
    }

    public function shutdown(): void {
        error_log("StatefulPlugin called {$this->callCount} times total");
    }

    public function process(object &$result, object $config): void {
        $this->callCount++;
        
        // Cache the last MIME type
        $this->cache['last_mime'] = $result->mime_type;
        $this->cache['last_timestamp'] = time();
        
        // Add cache info to metadata
        if (!isset($result->metadata)) {
            $result->metadata = [];
        }
        
        if (is_array($result->metadata)) {
            $result->metadata['plugin_call_count'] = $this->callCount;
            $result->metadata['cached_mime'] = $this->cache['last_mime'] ?? 'none';
        }
    }

    public function processingStage(): string {
        return "Middle";
    }

    public function shouldProcess(object $result, object $config): bool {
        // Always process to track state
        return true;
    }

    public function estimatedDurationMs(object $result): int {
        // State tracking is minimal overhead
        return 2;
    }

    public function priority(): int {
        return 50;
    }

    public function getCallCount(): int {
        return $this->callCount;
    }

    public function getCache(): array {
        return $this->cache;
    }
}

// Register the stateful plugin
$plugin = new StatefulPlugin();
Kreuzberg::registerPostProcessor($plugin);

echo "Stateful plugin registered\n";
// Can later retrieve state: $plugin->getCallCount(), $plugin->getCache()
```
