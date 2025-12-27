```php
<?php

declare(strict_types=1);

/**
 * Performance Tuning and Optimization
 *
 * Optimize document extraction for speed and resource usage.
 * Tips and techniques for processing large volumes of documents.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use function Kreuzberg\extract_file;
use function Kreuzberg\batch_extract_files;

function benchmark(callable $fn, string $label): void
{
    $startTime = microtime(true);
    $startMemory = memory_get_usage();

    $result = $fn();

    $elapsed = microtime(true) - $startTime;
    $memoryUsed = memory_get_usage() - $startMemory;

    echo "$label:\n";
    echo "  Time: " . number_format($elapsed, 4) . "s\n";
    echo "  Memory: " . number_format($memoryUsed / 1024 / 1024, 2) . " MB\n";
    echo "  Peak memory: " . number_format(memory_get_peak_usage() / 1024 / 1024, 2) . " MB\n\n";
}

$files = array_filter(
    ['doc1.pdf', 'doc2.pdf', 'doc3.pdf', 'doc4.pdf', 'doc5.pdf'],
    'file_exists'
);

if (!empty($files)) {
    echo "Performance Comparison:\n";
    echo str_repeat('=', 60) . "\n\n";

    benchmark(function () use ($files) {
        $results = [];
        foreach ($files as $file) {
            $results[] = extract_file($file);
        }
        return $results;
    }, "Single file processing");

    benchmark(function () use ($files) {
        return batch_extract_files($files);
    }, "Batch processing (parallel)");
}

$fastConfig = new ExtractionConfig(
    extractImages: false,     
    extractTables: false,     
    preserveFormatting: false 
);

$standardConfig = new ExtractionConfig(
    extractImages: true,
    extractTables: true,
    preserveFormatting: true
);

$testFile = 'large_document.pdf';
if (file_exists($testFile)) {
    echo "Configuration Impact:\n";
    echo str_repeat('=', 60) . "\n\n";

    benchmark(function () use ($testFile, $fastConfig) {
        $kreuzberg = new Kreuzberg($fastConfig);
        return $kreuzberg->extractFile($testFile);
    }, "Fast config (minimal features)");

    benchmark(function () use ($testFile, $standardConfig) {
        $kreuzberg = new Kreuzberg($standardConfig);
        return $kreuzberg->extractFile($testFile);
    }, "Standard config (all features)");
}

function processLargeDocumentEfficiently(string $filePath): void
{
    $config = new ExtractionConfig(
        page: new \Kreuzberg\Config\PageConfig(
            extractPages: true  
        ),
        extractImages: false    
    );

    $kreuzberg = new Kreuzberg($config);
    $result = $kreuzberg->extractFile($filePath);

    echo "Processing large document page by page:\n";

    foreach ($result->pages ?? [] as $page) {
        $pageContent = $page->content;

        unset($pageContent);

        echo "  Processed page {$page->pageNumber}\n";
    }

    unset($result);
    gc_collect_cycles();
}

if (file_exists('huge_document.pdf')) {
    processLargeDocumentEfficiently('huge_document.pdf');
}

function findOptimalBatchSize(array $files): int
{
    $batchSizes = [1, 5, 10, 20, 50];
    $results = [];

    foreach ($batchSizes as $size) {
        $batches = array_chunk($files, $size);
        $startTime = microtime(true);

        foreach ($batches as $batch) {
            batch_extract_files($batch);
        }

        $elapsed = microtime(true) - $startTime;
        $throughput = count($files) / $elapsed;

        $results[$size] = $throughput;

        echo "Batch size $size: " . number_format($throughput, 2) . " files/sec\n";
    }

    arsort($results);
    return array_key_first($results);
}

if (!empty($files) && count($files) >= 5) {
    echo "\nFinding optimal batch size:\n";
    echo str_repeat('=', 60) . "\n";
    $optimalSize = findOptimalBatchSize($files);
    echo "\nOptimal batch size: $optimalSize\n\n";
}

class ResourceMonitor
{
    private float $startTime;
    private int $startMemory;
    private array $checkpoints = [];

    public function __construct()
    {
        $this->startTime = microtime(true);
        $this->startMemory = memory_get_usage();
    }

    public function checkpoint(string $label): void
    {
        $this->checkpoints[] = [
            'label' => $label,
            'time' => microtime(true) - $this->startTime,
            'memory' => memory_get_usage() - $this->startMemory,
            'peak' => memory_get_peak_usage(),
        ];
    }

    public function report(): void
    {
        echo "Resource Monitor Report:\n";
        echo str_repeat('=', 60) . "\n";

        foreach ($this->checkpoints as $checkpoint) {
            printf("%-30s | Time: %6.3fs | Mem: %6.2f MB\n",
                $checkpoint['label'],
                $checkpoint['time'],
                $checkpoint['memory'] / 1024 / 1024
            );
        }

        echo "\nPeak memory: " . number_format(
            memory_get_peak_usage() / 1024 / 1024, 2
        ) . " MB\n";
    }
}

$monitor = new ResourceMonitor();

$kreuzberg = new Kreuzberg();
$monitor->checkpoint("Kreuzberg initialized");

$result = $kreuzberg->extractFile('document.pdf');
$monitor->checkpoint("Document extracted");

$words = str_word_count($result->content);
$monitor->checkpoint("Word count completed");

unset($result);
gc_collect_cycles();
$monitor->checkpoint("Memory freed");

$monitor->report();

function processConcurrently(array $files, int $workers = 4): array
{
    $chunks = array_chunk($files, ceil(count($files) / $workers));
    $results = [];

    foreach ($chunks as $chunk) {
        $chunkResults = batch_extract_files($chunk);
        $results = array_merge($results, $chunkResults);
    }

    return $results;
}

class CachedKreuzberg
{
    private array $cache = [];
    private int $maxCacheSize;

    public function __construct(
        private Kreuzberg $kreuzberg,
        int $maxCacheSize = 100
    ) {
        $this->maxCacheSize = $maxCacheSize;
    }

    public function extractFile(string $filePath): \Kreuzberg\Types\ExtractionResult
    {
        $cacheKey = md5($filePath . filemtime($filePath));

        if (isset($this->cache[$cacheKey])) {
            return $this->cache[$cacheKey];
        }

        $result = $this->kreuzberg->extractFile($filePath);

        if (count($this->cache) >= $this->maxCacheSize) {
            array_shift($this->cache); 
        }

        $this->cache[$cacheKey] = $result;
        return $result;
    }

    public function clearCache(): void
    {
        $this->cache = [];
    }
}

$cachedKreuzberg = new CachedKreuzberg(new Kreuzberg(), maxCacheSize: 50);

echo "\nCached extraction performance:\n";
echo str_repeat('=', 60) . "\n";

$file = 'document.pdf';
if (file_exists($file)) {
    benchmark(function () use ($cachedKreuzberg, $file) {
        return $cachedKreuzberg->extractFile($file);
    }, "First extraction (uncached)");

    benchmark(function () use ($cachedKreuzberg, $file) {
        return $cachedKreuzberg->extractFile($file);
    }, "Second extraction (cached)");
}

echo "\nPerformance Tips:\n";
echo str_repeat('=', 60) . "\n";
echo "1. Use batch processing for multiple files\n";
echo "2. Disable features you don't need (images, tables, OCR)\n";
echo "3. Process pages individually for very large documents\n";
echo "4. Use appropriate batch sizes (test to find optimal)\n";
echo "5. Implement caching for frequently accessed documents\n";
echo "6. Monitor memory usage and clear results when done\n";
echo "7. Consider using worker processes for high throughput\n";
echo "8. Increase PHP memory_limit for large documents\n";
```
