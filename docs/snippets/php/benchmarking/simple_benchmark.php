```php
<?php

declare(strict_types=1);

/**
 * Simple Benchmarking
 *
 * Benchmark document extraction performance across different
 * file types, sizes, and configurations.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use function Kreuzberg\extract_file;
use function Kreuzberg\batch_extract_files;

class Benchmark
{
    private array $results = [];

    public function run(string $name, callable $fn, int $iterations = 1): void
    {
        $times = [];
        $memories = [];

        for ($i = 0; $i < $iterations; $i++) {
            gc_collect_cycles();
            $startMemory = memory_get_usage();
            $startTime = microtime(true);

            $fn();

            $elapsed = microtime(true) - $startTime;
            $memoryUsed = memory_get_usage() - $startMemory;

            $times[] = $elapsed;
            $memories[] = $memoryUsed;
        }

        $this->results[$name] = [
            'iterations' => $iterations,
            'avg_time' => array_sum($times) / count($times),
            'min_time' => min($times),
            'max_time' => max($times),
            'avg_memory' => array_sum($memories) / count($memories),
            'peak_memory' => memory_get_peak_usage(),
        ];
    }

    public function report(): void
    {
        echo "Benchmark Results:\n";
        echo str_repeat('=', 80) . "\n\n";

        foreach ($this->results as $name => $stats) {
            echo "$name:\n";
            echo "  Iterations: {$stats['iterations']}\n";
            echo "  Average time: " . number_format($stats['avg_time'], 4) . "s\n";
            echo "  Min time: " . number_format($stats['min_time'], 4) . "s\n";
            echo "  Max time: " . number_format($stats['max_time'], 4) . "s\n";
            echo "  Average memory: " . number_format($stats['avg_memory'] / 1024 / 1024, 2) . " MB\n";
            echo "  Peak memory: " . number_format($stats['peak_memory'] / 1024 / 1024, 2) . " MB\n";
            echo "\n";
        }
    }

    public function compare(): void
    {
        if (count($this->results) < 2) {
            return;
        }

        echo "Performance Comparison:\n";
        echo str_repeat('=', 80) . "\n\n";

        $baseline = array_values($this->results)[0];
        $baselineName = array_keys($this->results)[0];

        foreach ($this->results as $name => $stats) {
            if ($name === $baselineName) continue;

            $speedup = $baseline['avg_time'] / $stats['avg_time'];
            $memoryRatio = $stats['avg_memory'] / $baseline['avg_memory'];

            echo "$name vs $baselineName:\n";
            echo "  Speed: " . number_format($speedup, 2) . "x ";
            echo ($speedup > 1 ? "faster" : "slower") . "\n";
            echo "  Memory: " . number_format($memoryRatio, 2) . "x ";
            echo ($memoryRatio < 1 ? "less" : "more") . "\n\n";
        }
    }
}

$benchmark = new Benchmark();

$testFile = 'test_document.pdf';
if (file_exists($testFile)) {
    $benchmark->run('Simple PDF extraction', function () use ($testFile) {
        extract_file($testFile);
    }, 5);
}

if (file_exists($testFile)) {
    $benchmark->run('PDF with table extraction', function () use ($testFile) {
        $config = new ExtractionConfig(extractTables: true);
        $kreuzberg = new Kreuzberg($config);
        $kreuzberg->extractFile($testFile);
    }, 5);
}

if (file_exists($testFile)) {
    $benchmark->run('PDF with OCR', function () use ($testFile) {
        $config = new ExtractionConfig(
            ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
        );
        $kreuzberg = new Kreuzberg($config);
        $kreuzberg->extractFile($testFile);
    }, 3);
}

$files = array_filter(['doc1.pdf', 'doc2.pdf', 'doc3.pdf'], 'file_exists');
if (count($files) >= 3) {
    $benchmark->run('Batch processing (3 files)', function () use ($files) {
        batch_extract_files(array_slice($files, 0, 3));
    }, 3);

    $benchmark->run('Sequential processing (3 files)', function () use ($files) {
        foreach (array_slice($files, 0, 3) as $file) {
            extract_file($file);
        }
    }, 3);
}

$fileTypes = [
    'PDF' => 'sample.pdf',
    'DOCX' => 'sample.docx',
    'XLSX' => 'sample.xlsx',
    'TXT' => 'sample.txt',
];

foreach ($fileTypes as $type => $file) {
    if (file_exists($file)) {
        $benchmark->run("$type extraction", function () use ($file) {
            extract_file($file);
        }, 5);
    }
}

$configs = [
    'Minimal' => new ExtractionConfig(
        extractTables: false,
        extractImages: false
    ),
    'Standard' => new ExtractionConfig(
        extractTables: true,
        extractImages: false
    ),
    'Full' => new ExtractionConfig(
        extractTables: true,
        extractImages: true,
        preserveFormatting: true
    ),
];

foreach ($configs as $name => $config) {
    if (file_exists($testFile)) {
        $benchmark->run("$name config", function () use ($testFile, $config) {
            $kreuzberg = new Kreuzberg($config);
            $kreuzberg->extractFile($testFile);
        }, 5);
    }
}

$benchmark->report();
$benchmark->compare();

echo "\nThroughput Test:\n";
echo str_repeat('=', 80) . "\n";

if (!empty($files)) {
    $start = microtime(true);
    $count = 0;

    foreach ($files as $file) {
        extract_file($file);
        $count++;
    }

    $elapsed = microtime(true) - $start;
    $throughput = $count / $elapsed;

    echo "Processed $count files in " . number_format($elapsed, 2) . " seconds\n";
    echo "Throughput: " . number_format($throughput, 2) . " files/second\n";
}

echo "\nMemory Stress Test:\n";
echo str_repeat('=', 80) . "\n";

$initialMemory = memory_get_usage();
$results = [];

for ($i = 0; $i < 10; $i++) {
    if (file_exists($testFile)) {
        $results[] = extract_file($testFile);
    }
}

$finalMemory = memory_get_usage();
$memoryGrowth = $finalMemory - $initialMemory;

echo "Processed 10 documents\n";
echo "Memory growth: " . number_format($memoryGrowth / 1024 / 1024, 2) . " MB\n";
echo "Average per document: " . number_format($memoryGrowth / 10 / 1024 / 1024, 2) . " MB\n";

unset($results);
gc_collect_cycles();

$afterCleanup = memory_get_usage();
echo "After cleanup: " . number_format($afterCleanup / 1024 / 1024, 2) . " MB\n";
```
