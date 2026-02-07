#!/usr/bin/env php
<?php
/**
 * Kreuzberg PHP extraction wrapper for benchmark harness.
 *
 * Supports two modes:
 * - sync: extract_file() - synchronous extraction (default)
 * - batch: batch_extract_files() - batch extraction for multiple files
 */

declare(strict_types=1);

$autoloadPaths = [
    __DIR__ . '/../../../packages/php/vendor/autoload.php',
    __DIR__ . '/../../../../packages/php/vendor/autoload.php',
];

$autoloaded = false;
foreach ($autoloadPaths as $autoloadPath) {
    if (file_exists($autoloadPath)) {
        require_once $autoloadPath;
        $autoloaded = true;
        break;
    }
}

if (!$autoloaded) {
    fwrite(STDERR, "Error: Could not find autoload.php. Run 'composer install' in packages/php/\n");
    exit(1);
}

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Exceptions\KreuzbergException;

define('DEBUG', getenv('KREUZBERG_BENCHMARK_DEBUG') === 'true');

/**
 * Log debug messages to stderr
 */
function debug_log(string $message): void
{
    if (!DEBUG) {
        return;
    }
    fwrite(STDERR, sprintf("[DEBUG] %s - %s\n", date('c'), $message));
}

/**
 * Extract a single file synchronously
 */
function extract_sync(string $filePath, ?ExtractionConfig $config = null): array
{
    debug_log("=== SYNC EXTRACTION START ===");
    debug_log("Input: file_path={$filePath}");
    debug_log("File exists: " . (file_exists($filePath) ? 'true' : 'false'));
    if (file_exists($filePath)) {
        debug_log("File size: " . filesize($filePath) . " bytes");
    }

    $start = microtime(true);
    debug_log("Timing start: {$start}");

    try {
        $result = Kreuzberg\extract_file($filePath, null, $config);
    } catch (KreuzbergException $e) {
        debug_log("ERROR during sync extraction: " . get_class($e) . " - " . $e->getMessage());
        throw $e;
    }

    $end = microtime(true);
    $durationMs = ($end - $start) * 1000.0;

    debug_log("Timing end: {$end}");
    debug_log("Duration (seconds): " . ($end - $start));
    debug_log("Duration (milliseconds): {$durationMs}");
    debug_log("Result has content: " . ($result->content !== null ? 'true' : 'false'));
    debug_log("Content length: " . strlen($result->content) . " characters");
    debug_log("Result has metadata: " . ($result->metadata !== null ? 'true' : 'false'));

    $payload = [
        'content' => $result->content,
        'metadata' => $result->metadata ?? [],
        '_extraction_time_ms' => $durationMs,
    ];

    debug_log("Output JSON size: " . strlen(json_encode($payload)) . " bytes");
    debug_log("=== SYNC EXTRACTION END ===");

    return $payload;
}

/**
 * Extract multiple files in batch
 */
function extract_batch(array $filePaths, ?ExtractionConfig $config = null): array
{
    debug_log("=== BATCH EXTRACTION START ===");
    debug_log("Input: " . count($filePaths) . " files");
    foreach ($filePaths as $idx => $path) {
        $exists = file_exists($path);
        $size = $exists ? filesize($path) : 'N/A';
        debug_log("  [{$idx}] {$path} (exists: " . ($exists ? 'true' : 'false') . ", size: {$size} bytes)");
    }

    $start = microtime(true);
    debug_log("Timing start: {$start}");

    try {
        $results = Kreuzberg\batch_extract_files($filePaths, $config);
    } catch (KreuzbergException $e) {
        debug_log("ERROR during batch extraction: " . get_class($e) . " - " . $e->getMessage());
        throw $e;
    }

    $end = microtime(true);
    $totalDurationMs = ($end - $start) * 1000.0;

    debug_log("Timing end: {$end}");
    debug_log("Total duration (seconds): " . ($end - $start));
    debug_log("Total duration (milliseconds): {$totalDurationMs}");
    debug_log("Results count: " . count($results));

    $perFileDurationMs = count($filePaths) > 0 ? $totalDurationMs / count($filePaths) : 0;
    debug_log("Per-file average duration (milliseconds): {$perFileDurationMs}");

    $resultsWithTiming = [];
    foreach ($results as $idx => $result) {
        debug_log("  Result[{$idx}] - content length: " . strlen($result->content) . ", has metadata: " . ($result->metadata !== null ? 'true' : 'false'));
        $resultsWithTiming[] = [
            'content' => $result->content,
            'metadata' => $result->metadata ?? [],
            '_extraction_time_ms' => $perFileDurationMs,
            '_batch_total_ms' => $totalDurationMs,
        ];
    }

    debug_log("=== BATCH EXTRACTION END ===");

    return $resultsWithTiming;
}

/**
 * Server mode: read paths from stdin, write JSON to stdout
 */
function run_server(?ExtractionConfig $config = null): void
{
    debug_log("=== SERVER MODE START ===");

    while (true) {
        $line = fgets(STDIN);
        if ($line === false) {
            break;
        }

        $filePath = trim($line);
        if (empty($filePath)) {
            continue;
        }

        debug_log("Processing file: {$filePath}");

        try {
            $start = microtime(true);
            $result = Kreuzberg\extract_file($filePath, null, $config);
            $durationMs = (microtime(true) - $start) * 1000.0;

            $payload = [
                'content' => $result->content,
                'metadata' => $result->metadata ?? [],
                '_extraction_time_ms' => $durationMs,
            ];

            echo json_encode($payload, JSON_THROW_ON_ERROR) . "\n";
            fflush(STDOUT);
        } catch (Throwable $e) {
            $errorPayload = [
                'error' => $e->getMessage(),
                '_extraction_time_ms' => 0,
            ];
            echo json_encode($errorPayload, JSON_THROW_ON_ERROR) . "\n";
            fflush(STDOUT);
        }
    }

    debug_log("=== SERVER MODE END ===");
}

/**
 * Main entry point
 */
function main(): void
{
    global $argv;

    debug_log("PHP script started");
    debug_log("ARGV: " . json_encode($argv));
    debug_log("ARGV length: " . count($argv));

    $ocrEnabled = false;
    $args = [];

    // Parse OCR flags
    for ($i = 1; $i < count($argv); $i++) {
        if ($argv[$i] === '--ocr') {
            $ocrEnabled = true;
        } elseif ($argv[$i] === '--no-ocr') {
            $ocrEnabled = false;
        } else {
            $args[] = $argv[$i];
        }
    }

    if (count($args) < 1) {
        fwrite(STDERR, "Usage: kreuzberg_extract.php [--ocr|--no-ocr] <mode> <file_path> [additional_files...]\n");
        fwrite(STDERR, "Modes: sync, batch, server\n");
        fwrite(STDERR, "Debug mode: set KREUZBERG_BENCHMARK_DEBUG=true to enable debug logging to stderr\n");
        exit(1);
    }

    $mode = $args[0];
    $filePaths = array_slice($args, 1);
    $config = new ExtractionConfig(
        useCache: false,
        ocr: $ocrEnabled ? new OcrConfig() : null,
    );

    debug_log("Mode: {$mode}");
    debug_log("OCR enabled: " . ($ocrEnabled ? 'true' : 'false'));
    debug_log("File paths (" . count($filePaths) . "): " . json_encode($filePaths));

    try {
        switch ($mode) {
            case 'server':
                debug_log("Executing server mode");
                run_server($config);
                break;

            case 'sync':
                if (count($filePaths) !== 1) {
                    fwrite(STDERR, "Error: sync mode requires exactly one file\n");
                    exit(1);
                }
                debug_log("Executing sync mode with file: {$filePaths[0]}");
                $payload = extract_sync($filePaths[0], $config);
                $output = json_encode($payload, JSON_THROW_ON_ERROR);
                debug_log("Output JSON: {$output}");
                echo $output;
                break;

            case 'batch':
                if (count($filePaths) < 1) {
                    fwrite(STDERR, "Error: batch mode requires at least one file\n");
                    exit(1);
                }
                debug_log("Executing batch mode with " . count($filePaths) . " files");

                $results = extract_batch($filePaths, $config);

                if (count($filePaths) === 1) {
                    $output = json_encode($results[0], JSON_THROW_ON_ERROR);
                    debug_log("Output JSON (single file): {$output}");
                    echo $output;
                } else {
                    $output = json_encode($results, JSON_THROW_ON_ERROR);
                    if (strlen($output) > 200) {
                        debug_log("Output JSON (multiple files): " . substr($output, 0, 200) . "...");
                    }
                    echo $output;
                }
                break;

            default:
                fwrite(STDERR, "Error: Unknown mode '{$mode}'. Use sync, batch, or server\n");
                exit(1);
        }

        debug_log("Script completed successfully");
    } catch (Throwable $e) {
        debug_log("FATAL ERROR: " . get_class($e) . " - " . $e->getMessage());
        debug_log("Backtrace:\n" . $e->getTraceAsString());
        fwrite(STDERR, "Error extracting with Kreuzberg: {$e->getMessage()}\n");
        exit(1);
    }
}

main();
