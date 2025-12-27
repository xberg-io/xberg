<?php

declare(strict_types=1);

namespace Kreuzberg\Plugins;

use Closure;

/**
 * Registry for managing custom document extractors.
 *
 * This class provides a convenient object-oriented interface for registering,
 * unregistering, and managing custom document extractors. It wraps the procedural
 * functions from the Kreuzberg extension.
 *
 * @package Kreuzberg\Plugins
 */
final class ExtractorRegistry
{
    /**
     * Register a custom extractor for a specific MIME type.
     *
     * The extractor will be called whenever a document with the specified MIME type
     * is processed. Custom extractors take precedence over built-in extractors.
     *
     * @param string $mimeType MIME type to handle (e.g., "text/custom", "application/x-special")
     * @param ExtractorInterface|callable $extractor Extractor instance or callable
     *
     * @throws \InvalidArgumentException If MIME type is empty or extractor is invalid
     *
     * @example Using an ExtractorInterface implementation
     * ```php
     * class CustomExtractor implements ExtractorInterface
     * {
     *     public function extract(string $bytes, string $mimeType): array
     *     {
     *         return [
     *             'content' => 'extracted text',
     *             'metadata' => [],
     *             'tables' => [],
     *         ];
     *     }
     * }
     *
     * ExtractorRegistry::register('text/custom', new CustomExtractor());
     * ```
     *
     * @example Using a closure
     * ```php
     * ExtractorRegistry::register('text/custom', function (string $bytes, string $mimeType): array {
     *     return [
     *         'content' => 'extracted text',
     *         'metadata' => [],
     *         'tables' => [],
     *     ];
     * });
     * ```
     */
    public static function register(string $mimeType, ExtractorInterface|callable $extractor): void
    {
        if (trim($mimeType) === '') {
            throw new \InvalidArgumentException('MIME type cannot be empty');
        }

        if ($extractor instanceof ExtractorInterface) {
            $callback = [$extractor, 'extract'];
        } else {
            $callback = $extractor;
        }

        kreuzberg_register_extractor($mimeType, $callback);
    }

    /**
     * Unregister a custom extractor for a specific MIME type.
     *
     * @param string $mimeType MIME type to unregister
     *
     * @example
     * ```php
     * ExtractorRegistry::unregister('text/custom');
     * ```
     */
    public static function unregister(string $mimeType): void
    {
        kreuzberg_unregister_extractor($mimeType);
    }

    /**
     * Get a list of all registered custom extractor MIME types.
     *
     * @return array<int, string> Array of MIME type strings
     *
     * @example
     * ```php
     * $mimeTypes = ExtractorRegistry::list();
     * foreach ($mimeTypes as $mimeType) {
     *     echo "Registered extractor: {$mimeType}\n";
     * }
     * ```
     */
    public static function list(): array
    {
        $result = kreuzberg_list_extractors();
        return array_values($result);
    }

    /**
     * Clear all registered custom extractors.
     *
     * This removes all custom extractors from the registry. Useful for cleanup
     * in tests or when resetting the extraction environment.
     *
     * @example
     * ```php
     * // In PHPUnit test tearDown
     * protected function tearDown(): void
     * {
     *     ExtractorRegistry::clear();
     * }
     * ```
     */
    public static function clear(): void
    {
        kreuzberg_clear_extractors();
    }

    /**
     * Check if a custom extractor is registered for a MIME type.
     *
     * @param string $mimeType MIME type to check
     * @return bool True if an extractor is registered, false otherwise
     *
     * @example
     * ```php
     * if (ExtractorRegistry::has('text/custom')) {
     *     echo "Custom extractor is registered\n";
     * }
     * ```
     */
    public static function has(string $mimeType): bool
    {
        return in_array($mimeType, self::list(), true);
    }

    /**
     * Test a plugin for compatibility.
     *
     * This validates plugin compatibility by testing a plugin file path.
     *
     * @param string $pluginPath Path to the plugin file to test
     *
     * @return bool True if plugin is compatible, false otherwise
     *
     * @example
     * ```php
     * if (ExtractorRegistry::test('/path/to/plugin.php')) {
     *     echo "Plugin is compatible\n";
     * }
     * ```
     */
    public static function test(string $pluginPath): bool
    {
        return kreuzberg_test_plugin($pluginPath);
    }
}
