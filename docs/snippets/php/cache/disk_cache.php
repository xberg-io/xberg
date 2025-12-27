```php
<?php

declare(strict_types=1);

/**
 * Disk Cache for Document Extraction
 *
 * Implement file-based caching to avoid re-processing the same documents.
 * Significantly improves performance for repeated extractions.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Types\ExtractionResult;

class DiskCache
{
    private string $cacheDir;
    private int $ttl;

    public function __construct(string $cacheDir = null, int $ttl = 7 * 86400)
    {
        $this->cacheDir = $cacheDir ?? sys_get_temp_dir() . '/kreuzberg_cache';
        $this->ttl = $ttl;

        if (!is_dir($this->cacheDir)) {
            mkdir($this->cacheDir, 0755, true);
        }
    }

    private function getCacheKey(string $filePath, ExtractionConfig $config): string
    {
        $fileHash = md5_file($filePath);
        $configHash = md5(json_encode($config->toArray()));
        return md5($filePath . $fileHash . $configHash);
    }

    private function getCachePath(string $key): string
    {
        return $this->cacheDir . '/' . $key . '.cache';
    }

    public function get(string $filePath, ExtractionConfig $config): ?ExtractionResult
    {
        $key = $this->getCacheKey($filePath, $config);
        $cachePath = $this->getCachePath($key);

        if (!file_exists($cachePath)) {
            return null;
        }

        if (time() - filemtime($cachePath) > $this->ttl) {
            unlink($cachePath);
            return null;
        }

        $data = file_get_contents($cachePath);
        if ($data === false) {
            return null;
        }

        $cached = unserialize($data);
        if ($cached instanceof ExtractionResult) {
            return $cached;
        }

        return null;
    }

    public function set(string $filePath, ExtractionConfig $config, ExtractionResult $result): void
    {
        $key = $this->getCacheKey($filePath, $config);
        $cachePath = $this->getCachePath($key);

        file_put_contents($cachePath, serialize($result));
    }

    public function clear(): void
    {
        $files = glob($this->cacheDir . '/*.cache');
        foreach ($files as $file) {
            unlink($file);
        }
    }

    public function getStats(): array
    {
        $files = glob($this->cacheDir . '/*.cache');
        $totalSize = 0;

        foreach ($files as $file) {
            $totalSize += filesize($file);
        }

        return [
            'total_entries' => count($files),
            'cache_size_bytes' => $totalSize,
            'cache_dir' => $this->cacheDir,
        ];
    }
}

$cache = new DiskCache();
$kreuzberg = new Kreuzberg();
$config = new ExtractionConfig();

$file = 'document.pdf';

echo "First extraction (will be cached)...\n";
$start = microtime(true);

$result = $cache->get($file, $config);

if ($result === null) {
    $result = $kreuzberg->extractFile($file, config: $config);
    $cache->set($file, $config, $result);
    echo "  Status: Extracted and cached\n";
} else {
    echo "  Status: Retrieved from cache\n";
}

$elapsed = microtime(true) - $start;
echo "  Time: " . number_format($elapsed, 4) . "s\n";
echo "  Content length: " . strlen($result->content) . " chars\n\n";

echo "Second extraction (from cache)...\n";
$start = microtime(true);

$result = $cache->get($file, $config);

if ($result === null) {
    $result = $kreuzberg->extractFile($file, config: $config);
    $cache->set($file, $config, $result);
    echo "  Status: Extracted and cached\n";
} else {
    echo "  Status: Retrieved from cache\n";
}

$elapsed = microtime(true) - $start;
echo "  Time: " . number_format($elapsed, 4) . "s\n";
echo "  Content length: " . strlen($result->content) . " chars\n\n";

$stats = $cache->getStats();
echo "Cache Statistics:\n";
echo str_repeat('=', 60) . "\n";
echo "Total entries: {$stats['total_entries']}\n";
echo "Cache size: " . number_format($stats['cache_size_bytes'] / 1024 / 1024, 2) . " MB\n";
echo "Cache directory: {$stats['cache_dir']}\n\n";

class CachedKreuzberg
{
    public function __construct(
        private Kreuzberg $kreuzberg,
        private DiskCache $cache
    ) {}

    public function extractFile(
        string $filePath,
        ?string $mimeType = null,
        ?ExtractionConfig $config = null
    ): ExtractionResult {
        $config = $config ?? new ExtractionConfig();

        $result = $this->cache->get($filePath, $config);

        if ($result === null) {
            $result = $this->kreuzberg->extractFile($filePath, $mimeType, $config);
            $this->cache->set($filePath, $config, $result);
        }

        return $result;
    }

    public function clearCache(): void
    {
        $this->cache->clear();
    }

    public function getCacheStats(): array
    {
        return $this->cache->getStats();
    }
}

$cachedKreuzberg = new CachedKreuzberg(
    new Kreuzberg(),
    new DiskCache()
);

echo "Using CachedKreuzberg wrapper:\n";
echo str_repeat('=', 60) . "\n";

$files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf'];
foreach ($files as $file) {
    if (!file_exists($file)) continue;

    $start = microtime(true);
    $result = $cachedKreuzberg->extractFile($file);
    $elapsed = microtime(true) - $start;

    echo "$file: " . number_format($elapsed, 4) . "s\n";
}

echo "\nCache stats:\n";
$stats = $cachedKreuzberg->getCacheStats();
print_r($stats);

function cleanupCache(DiskCache $cache, int $maxAge = 7 * 86400): int
{
    $cacheDir = $cache->getStats()['cache_dir'];
    $files = glob($cacheDir . '/*.cache');
    $deleted = 0;

    foreach ($files as $file) {
        if (time() - filemtime($file) > $maxAge) {
            unlink($file);
            $deleted++;
        }
    }

    return $deleted;
}

$deleted = cleanupCache($cache, 7 * 86400);
echo "\nCleaned up $deleted old cache entries\n";
```
