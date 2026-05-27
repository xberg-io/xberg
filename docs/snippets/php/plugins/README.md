# PHP Plugin System - Deferred to Future Version

## Status: Not Yet Implemented

The PHP plugin system for Kreuzberg is **deferred to a future version**. This includes:

- Custom OCR backend registration
- Post-processor plugins
- Validator plugins
- Custom extractor plugins

## Why Deferred?

The plugin system requires complex callback handling between Rust and PHP through ext-php-rs. Specifically:

1. **Callback Challenges**: ext-php-rs callback support for complex interfaces is still evolving
2. **Memory Safety**: Ensuring proper lifetime management for PHP closures called from Rust
3. **Error Handling**: Propagating exceptions across the FFI boundary in plugin contexts
4. **Performance**: Minimizing overhead of cross-language callbacks in hot paths

## Affected Functions (~16 functions)

The following functions exist in Python, Ruby, Node.js, and other bindings but are not yet available in PHP:

### OCR Backend Registration

- `kreuzberg_register_ocr_backend()`
- `kreuzberg_unregister_ocr_backend()`
- `kreuzberg_list_ocr_backends()`

### Post-Processor Plugins

- `kreuzberg_register_post_processor()`
- `kreuzberg_unregister_post_processor()`
- `kreuzberg_list_post_processors()`
- `kreuzberg_clear_post_processors()`

### Validator Plugins

- `kreuzberg_register_validator()`
- `kreuzberg_unregister_validator()`
- `kreuzberg_list_validators()`
- `kreuzberg_clear_validators()`

### Custom Extractor Plugins

- `kreuzberg_register_extractor()`
- `kreuzberg_unregister_extractor()`
- `kreuzberg_list_extractors()`
- `kreuzberg_clear_extractors()`

### Plugin Testing

- `kreuzberg_test_plugin()`

## Workarounds

Until the plugin system is implemented, you can:

### 1. Post-Process Results in PHP

Instead of registering a post-processor plugin, process the extraction result directly:

```php title="Post-Process Results"
<?php

declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\Types\ExtractionResult;

function postProcessResult(ExtractionResult $result): ExtractionResult
{
    // Custom post-processing logic
    $processedContent = strtoupper($result->content);

    // Return a new result with modified content
    return new ExtractionResult(
        content: $processedContent,
        mimeType: $result->mimeType,
        metadata: $result->metadata,
        tables: $result->tables,
        images: $result->images,
        chunks: $result->chunks,
    );
}

$kreuzberg = new Kreuzberg();
$result = $kreuzberg->extractFile('document.pdf');
$processed = postProcessResult($result);
```

### 2. Use Built-in OCR Backends

PHP bindings support all built-in OCR backends:

```php title="Built-in OCR Backends"
<?php

declare(strict_types=1);

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Kreuzberg;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',  // Built-in: tesseract, apple-vision (macOS)
        language: 'eng',
    ),
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('scanned.pdf');
```

### 3. Validate Results in PHP

Instead of validator plugins, validate extraction results directly:

```php title="Validate Results"
<?php

declare(strict_types=1);

use Kreuzberg\Exceptions\ValidationException;
use Kreuzberg\Types\ExtractionResult;

function validateResult(ExtractionResult $result): void
{
    if (strlen($result->content) < 100) {
        throw new ValidationException('Content too short (minimum 100 characters)');
    }

    if ($result->metadata?->pageCount === 0) {
        throw new ValidationException('Document has no pages');
    }
}

$result = $kreuzberg->extractFile('document.pdf');
validateResult($result);
```

### 4. Extend the Kreuzberg Class

For application-specific functionality, extend the main class:

```php title="Extend Kreuzberg Class"
<?php

declare(strict_types=1);

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Kreuzberg as BaseKreuzberg;
use Kreuzberg\Types\ExtractionResult;

final class CustomKreuzberg extends BaseKreuzberg
{
    public function extractAndValidate(
        string $path,
        ?ExtractionConfig $config = null
    ): ExtractionResult {
        $result = $this->extractFile($path, $config);

        // Custom validation
        if (strlen($result->content) < 100) {
            throw new \RuntimeException('Content too short');
        }

        return $result;
    }

    public function extractAndTransform(
        string $path,
        callable $transformer,
        ?ExtractionConfig $config = null
    ): ExtractionResult {
        $result = $this->extractFile($path, $config);

        // Custom transformation
        $transformedContent = $transformer($result->content);

        return new ExtractionResult(
            content: $transformedContent,
            mimeType: $result->mimeType,
            metadata: $result->metadata,
            tables: $result->tables,
            images: $result->images,
            chunks: $result->chunks,
        );
    }
}
```

## Timeline

The plugin system is planned for a future PHP bindings release (tentatively v4.1.0 or v4.2.0), pending:

1. Ext-php-rs improvements for complex callbacks
2. Comprehensive testing of callback performance and safety
3. Documentation of plugin interfaces

## Current Feature Parity

Despite the deferred plugin system, PHP bindings achieve **95% feature parity** with other language bindings:

- ✅ All extraction functions (file, bytes, batch)
- ✅ All configuration options (OCR, PDF, chunking, embeddings)
- ✅ All result types (tables, images, chunks, metadata)
- ✅ All validation functions (14 validators)
- ✅ Embedding presets (2 functions + class)
- ✅ Error classification (3 functions + class)
- ✅ Config helpers (JSON export, field access, merging)
- ❌ Plugin system (16 functions) - **deferred**

## Questions?

For questions about the plugin system or to request early access when available:

- GitHub Issues: <https://github.com/kreuzberg-dev/kreuzberg/issues>
- Discussions: <https://github.com/kreuzberg-dev/kreuzberg/discussions>

## Contributing

If you're interested in helping implement the plugin system for PHP:

1. Review the plugin implementations in Python (`crates/kreuzberg-py/src/plugins.rs`)
2. Review ext-php-rs callback documentation
3. Open a discussion on the Kreuzberg GitHub repository

We welcome contributions!
