<?php

declare(strict_types=1);

/**
 * Mock implementation of Kreuzberg extension functions for testing.
 *
 * This provides PHP implementations of the extension functions when the
 * Rust extension is not available, allowing tests to run.
 */

// Only define if not already defined by the extension
if (!function_exists('kreuzberg_extract_file')) {
    /**
     * @param array<string, mixed>|null $config
     * @return array<string, mixed>
     */
    function kreuzberg_extract_file(
        string $filePath,
        ?string $mimeType = null,
        ?array $config = null,
    ): array {
        // Mock implementation - return basic extraction result
        if (!file_exists($filePath)) {
            throw new \Kreuzberg\Exceptions\KreuzbergException("File not found: $filePath");
        }

        $content = file_get_contents($filePath);
        if ($content === false) {
            throw new \Kreuzberg\Exceptions\KreuzbergException("Failed to read file: $filePath");
        }

        return [
            'content' => "Mock extraction result from $filePath",
            'mime_type' => $mimeType ?? 'application/octet-stream',
            'metadata' => [],
            'tables' => [],
            'detected_languages' => ['en'],
            'chunks' => [],
            'images' => [],
            'pages' => [],
            'embeddings' => [],
            'keywords' => [],
            'tesseract' => [],
        ];
    }
}

if (!function_exists('kreuzberg_extract_bytes')) {
    /**
     * @param array<string, mixed>|null $config
     * @return array<string, mixed>
     */
    function kreuzberg_extract_bytes(
        string $data,
        string $mimeType,
        ?array $config = null,
    ): array {
        // Mock implementation
        return [
            'content' => 'Mock extraction result from bytes',
            'mime_type' => $mimeType,
            'metadata' => [],
            'tables' => [],
            'detected_languages' => ['en'],
            'chunks' => [],
            'images' => [],
            'pages' => [],
            'embeddings' => [],
            'keywords' => [],
            'tesseract' => [],
        ];
    }
}

if (!function_exists('kreuzberg_batch_extract_files')) {
    /**
     * @param array<int, string> $paths
     * @param array<string, mixed>|null $config
     * @return array<int, array<string, mixed>>
     */
    function kreuzberg_batch_extract_files(
        array $paths,
        ?array $config = null,
    ): array {
        // Mock implementation
        $results = [];
        foreach ($paths as $path) {
            $results[] = [
                'content' => "Mock extraction from $path",
                'mime_type' => 'application/octet-stream',
                'metadata' => [],
                'tables' => [],
                'detected_languages' => ['en'],
                'chunks' => [],
                'images' => [],
                'pages' => [],
                'embeddings' => [],
                'keywords' => [],
                'tesseract' => [],
            ];
        }
        return $results;
    }
}

if (!function_exists('kreuzberg_batch_extract_bytes')) {
    /**
     * @param array<int, string> $dataList
     * @param array<int, string> $mimeTypes
     * @param array<string, mixed>|null $config
     * @return array<int, array<string, mixed>>
     */
    function kreuzberg_batch_extract_bytes(
        array $dataList,
        array $mimeTypes,
        ?array $config = null,
    ): array {
        // Mock implementation
        $results = [];
        foreach ($dataList as $index => $data) {
            $results[] = [
                'content' => "Mock extraction result $index",
                'mime_type' => $mimeTypes[$index] ?? 'application/octet-stream',
                'metadata' => [],
                'tables' => [],
                'detected_languages' => ['en'],
                'chunks' => [],
                'images' => [],
                'pages' => [],
                'embeddings' => [],
                'keywords' => [],
                'tesseract' => [],
            ];
        }
        return $results;
    }
}

if (!function_exists('kreuzberg_detect_mime_type')) {
    function kreuzberg_detect_mime_type(string $data): string
    {
        // Mock implementation - simple magic number detection
        if (str_starts_with($data, '%PDF')) {
            return 'application/pdf';
        }
        if (str_starts_with($data, "\x89PNG")) {
            return 'image/png';
        }
        if (str_starts_with($data, "\xFF\xD8\xFF")) {
            return 'image/jpeg';
        }
        if (str_starts_with($data, 'PK')) {
            return 'application/zip';
        }
        return 'application/octet-stream';
    }
}

if (!function_exists('kreuzberg_detect_mime_type_from_bytes')) {
    function kreuzberg_detect_mime_type_from_bytes(string $data): string
    {
        return kreuzberg_detect_mime_type($data);
    }
}

if (!function_exists('kreuzberg_detect_mime_type_from_path')) {
    function kreuzberg_detect_mime_type_from_path(string $path): string
    {
        $data = file_get_contents($path, false, null, 0, 512);
        if ($data === false) {
            return 'application/octet-stream';
        }
        return kreuzberg_detect_mime_type($data);
    }
}

if (!function_exists('kreuzberg_register_post_processor')) {
    function kreuzberg_register_post_processor(string $name, callable $callback): void
    {
        // Mock implementation
    }
}

if (!function_exists('kreuzberg_unregister_post_processor')) {
    function kreuzberg_unregister_post_processor(string $name): void
    {
        // Mock implementation
    }
}

if (!function_exists('kreuzberg_list_post_processors')) {
    /**
     * @return list<string>
     */
    function kreuzberg_list_post_processors(): array
    {
        return [];
    }
}

if (!function_exists('kreuzberg_clear_post_processors')) {
    function kreuzberg_clear_post_processors(): void
    {
        // Mock implementation
    }
}

if (!function_exists('kreuzberg_run_post_processors')) {
    function kreuzberg_run_post_processors(mixed &$result): void
    {
        // Mock implementation
    }
}

if (!function_exists('kreuzberg_register_validator')) {
    function kreuzberg_register_validator(string $name, callable $callback): void
    {
        // Mock implementation
    }
}

if (!function_exists('kreuzberg_unregister_validator')) {
    function kreuzberg_unregister_validator(string $name): void
    {
        // Mock implementation
    }
}

if (!function_exists('kreuzberg_list_validators')) {
    /**
     * @return list<string>
     */
    function kreuzberg_list_validators(): array
    {
        return [];
    }
}

if (!function_exists('kreuzberg_clear_validators')) {
    function kreuzberg_clear_validators(): void
    {
        // Mock implementation
    }
}

if (!function_exists('kreuzberg_run_validators')) {
    function kreuzberg_run_validators(mixed &$result): void
    {
        // Mock implementation
    }
}

if (!function_exists('kreuzberg_register_extractor')) {
    function kreuzberg_register_extractor(string $mimeType, callable $callback): void
    {
        // Mock implementation
    }
}

if (!function_exists('kreuzberg_unregister_extractor')) {
    function kreuzberg_unregister_extractor(string $mimeType): void
    {
        // Mock implementation
    }
}

if (!function_exists('kreuzberg_list_extractors')) {
    /**
     * @return list<string>
     */
    function kreuzberg_list_extractors(): array
    {
        return [];
    }
}

if (!function_exists('kreuzberg_clear_extractors')) {
    function kreuzberg_clear_extractors(): void
    {
        // Mock implementation
    }
}

if (!function_exists('kreuzberg_test_plugin')) {
    function kreuzberg_test_plugin(string $pluginType, string $pluginName, mixed &$testData): bool
    {
        return true;
    }
}

if (!function_exists('kreuzberg_list_embedding_presets')) {
    /**
     * @return array<string, array{model: string, dimensions: int}>
     */
    function kreuzberg_list_embedding_presets(): array
    {
        return ['default' => ['model' => 'default', 'dimensions' => 384]];
    }
}

if (!function_exists('kreuzberg_get_embedding_preset')) {
    /**
     * @return array{model: string, dimensions: int}|null
     */
    function kreuzberg_get_embedding_preset(string $name): ?array
    {
        $presets = kreuzberg_list_embedding_presets();
        return $presets[$name] ?? null;
    }
}
