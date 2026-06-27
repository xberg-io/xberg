# PHP Plugin System - Deferred to Future Version

## Status: Not Yet Implemented

The PHP plugin system for Xberg is **deferred to a future version**. This includes:

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

- `xberg_register_ocr_backend()`
- `xberg_unregister_ocr_backend()`
- `xberg_list_ocr_backends()`

### Post-Processor Plugins

- `xberg_register_post_processor()`
- `xberg_unregister_post_processor()`
- `xberg_list_post_processors()`
- `xberg_clear_post_processors()`

### Validator Plugins

- `xberg_register_validator()`
- `xberg_unregister_validator()`
- `xberg_list_validators()`
- `xberg_clear_validators()`

### Custom Extractor Plugins

- `xberg_register_extractor()`
- `xberg_unregister_extractor()`
- `xberg_list_extractors()`
- `xberg_clear_extractors()`

### Plugin Testing

- `xberg_test_plugin()`

## Workarounds

Until the plugin system is implemented, you can:

### 1. Post-Process Results in PHP

Instead of registering a post-processor plugin, process the extraction result directly:

```php title="Post-Process Results"
<?php

declare(strict_types=1);

use Xberg\Xberg;
use Xberg\Types\ExtractionResult;

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

$xberg = new Xberg();
$result = $xberg->extract('document.pdf');
$processed = postProcessResult($result);
```

### 2. Use Built-in OCR Backends

PHP bindings support all built-in OCR backends:

```php title="Built-in OCR Backends"
<?php

declare(strict_types=1);

use Xberg\Config\ExtractionConfig;
use Xberg\Config\OcrConfig;
use Xberg\Xberg;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',  // Built-in: tesseract, apple-vision (macOS)
        language: 'eng',
    ),
);

$xberg = new Xberg($config);
$result = $xberg->extract('scanned.pdf');
```

### 3. Validate Results in PHP

Instead of validator plugins, validate extraction results directly:

```php title="Validate Results"
<?php

declare(strict_types=1);

use Xberg\Exceptions\ValidationException;
use Xberg\Types\ExtractionResult;

function validateResult(ExtractionResult $result): void
{
    if (strlen($result->content) < 100) {
        throw new ValidationException('Content too short (minimum 100 characters)');
    }

    if ($result->metadata?->pageCount === 0) {
        throw new ValidationException('Document has no pages');
    }
}

$result = $xberg->extract('document.pdf');
validateResult($result);
```

### 4. Extend the Xberg Class

For application-specific functionality, extend the main class:

```php title="Extend Xberg Class"
<?php

declare(strict_types=1);

use Xberg\Config\ExtractionConfig;
use Xberg\Xberg as BaseXberg;
use Xberg\Types\ExtractionResult;

final class CustomXberg extends BaseXberg
{
    public function extractAndValidate(
        string $path,
        ?ExtractionConfig $config = null
    ): ExtractionResult {
        $result = $this->extract($path, $config);

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
        $result = $this->extract($path, $config);

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

- GitHub Issues: <https://github.com/xberg-io/xberg/issues>
- Discussions: <https://github.com/xberg-io/xberg/discussions>

## Contributing

If you're interested in helping implement the plugin system for PHP:

1. Review the plugin implementations in Python (`crates/xberg-py/src/plugins.rs`)
2. Review ext-php-rs callback documentation
3. Open a discussion on the Xberg GitHub repository

We welcome contributions!
