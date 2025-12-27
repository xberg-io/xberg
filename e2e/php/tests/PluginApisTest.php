<?php

declare(strict_types=1);

// Auto-generated from fixtures/plugin_api/ - DO NOT EDIT

/**
 * E2E tests for plugin/config/utility APIs.
 *
 * Generated from plugin API fixtures.
 * To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang php
 */

namespace E2EPhp\Tests;

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use PHPUnit\Framework\TestCase;

class PluginApisTest extends TestCase
{
    /**
     * Discover configuration from current or parent directories
     */
    public function test_config_discover(): void
    {
        $tmpDir = sys_get_temp_dir() . '/config_discover_' . uniqid();
        mkdir($tmpDir);
        $configPath = $tmpDir . '/' . 'kreuzberg.toml';
        file_put_contents($configPath, '[chunking]\nmax_chars = 50\n');

        $subdir = $tmpDir . '/' . 'subdir';
        mkdir($subdir);
        $oldCwd = getcwd();
        chdir($subdir);

        $config = ExtractionConfig::discover();
        $this->assertNotNull($config);

        $this->assertNotNull($config->chunking);
        $this->assertEquals(50, $config->chunking->max_chars);
        chdir($oldCwd);
        unlink($configPath);
        rmdir($subdir);
        rmdir($tmpDir);
    }

    /**
     * Load configuration from a TOML file
     */
    public function test_config_from_file(): void
    {
        $tmpDir = sys_get_temp_dir();
        $configPath = $tmpDir . '/' . 'test_config.toml';
        file_put_contents($configPath, '[chunking]\nmax_chars = 100\nmax_overlap = 20\n\n[language_detection]\nenabled = false\n');

        $config = ExtractionConfig::fromFile($configPath);

        $this->assertNotNull($config->chunking);
        $this->assertEquals(100, $config->chunking->max_chars);
        $this->assertEquals(20, $config->chunking->max_overlap);
        $this->assertNotNull($config->language_detection);
        $this->assertEquals(false, $config->language_detection->enabled);
        unlink($configPath);
    }

    /**
     * Clear all document extractors and verify list is empty
     */
    public function test_extractors_clear(): void
    {
        Kreuzberg::cleardocumentextractors();
        $result = Kreuzberg::listdocumentextractors();
        $this->assertEmpty($result);
    }

    /**
     * List all registered document extractors
     */
    public function test_extractors_list(): void
    {
        $result = Kreuzberg::listdocumentextractors();
        $this->assertIsArray($result);
        foreach ($result as $item) {
            $this->assertIsString($item);
        }
    }

    /**
     * Unregister nonexistent document extractor gracefully
     */
    public function test_extractors_unregister(): void
    {
        Kreuzberg::unregisterdocumentextractor('nonexistent-extractor-xyz');
        $this->assertTrue(true);
    }

    /**
     * Detect MIME type from file bytes
     */
    public function test_mime_detect_bytes(): void
    {
        $testBytes = '%PDF-1.4\\n';
        $result = Kreuzberg::detectmimetype($testBytes);

        $this->assertStringContainsStringIgnoringCase('pdf', $result);
    }

    /**
     * Detect MIME type from file path
     */
    public function test_mime_detect_path(): void
    {
        $tmpDir = sys_get_temp_dir();
        $testFile = $tmpDir . '/' . 'test.txt';
        file_put_contents($testFile, 'Hello, world!');

        $result = Kreuzberg::detectmimetypefrompath($testFile);

        $this->assertStringContainsStringIgnoringCase('text', $result);
        unlink($testFile);
    }

    /**
     * Get file extensions for a MIME type
     */
    public function test_mime_get_extensions(): void
    {
        $result = Kreuzberg::getextensionsformime('application/pdf');
        $this->assertIsArray($result);
        $this->assertContains('pdf', $result);
    }

    /**
     * Clear all OCR backends and verify list is empty
     */
    public function test_ocr_backends_clear(): void
    {
        Kreuzberg::clearocrbackends();
        $result = Kreuzberg::listocrbackends();
        $this->assertEmpty($result);
    }

    /**
     * List all registered OCR backends
     */
    public function test_ocr_backends_list(): void
    {
        $result = Kreuzberg::listocrbackends();
        $this->assertIsArray($result);
        foreach ($result as $item) {
            $this->assertIsString($item);
        }
    }

    /**
     * Unregister nonexistent OCR backend gracefully
     */
    public function test_ocr_backends_unregister(): void
    {
        Kreuzberg::unregisterocrbackend('nonexistent-backend-xyz');
        $this->assertTrue(true);
    }

    /**
     * Clear all post-processors and verify list is empty
     */
    public function test_post_processors_clear(): void
    {
        Kreuzberg::clearpostprocessors();
        $result = Kreuzberg::listpostprocessors();
        $this->assertEmpty($result);
    }

    /**
     * List all registered post-processors
     */
    public function test_post_processors_list(): void
    {
        $result = Kreuzberg::listpostprocessors();
        $this->assertIsArray($result);
        foreach ($result as $item) {
            $this->assertIsString($item);
        }
    }

    /**
     * Clear all validators and verify list is empty
     */
    public function test_validators_clear(): void
    {
        Kreuzberg::clearvalidators();
        $result = Kreuzberg::listvalidators();
        $this->assertEmpty($result);
    }

    /**
     * List all registered validators
     */
    public function test_validators_list(): void
    {
        $result = Kreuzberg::listvalidators();
        $this->assertIsArray($result);
        foreach ($result as $item) {
            $this->assertIsString($item);
        }
    }

}
