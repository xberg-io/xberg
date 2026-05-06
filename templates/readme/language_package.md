# {{ name }}

{% include 'partials/badges.html.jinja' %}

{{ description }}

## Installation

{% include 'partials/installation.md.jinja' %}

## Quick Start

{% include 'partials/quick_start.md.jinja' %}

{% if language == "typescript" %}
{% include 'partials/napi_implementation.md.jinja' %}

{% endif %}

## Features

{% include 'partials/features.md.jinja' %}

{% if features.ocr %}

## OCR Support

Kreuzberg supports multiple OCR backends for extracting text from scanned documents and images:

{% for backend in ocr_backends %}

- **{{ backend | title }}**
  {% endfor %}

### OCR Configuration Example

{{ snippets.ocr_configuration | include_snippet(language) }}

{% endif %}
{% if features.async %}

## Async Support

This binding provides full async/await support for non-blocking document processing:

{{ snippets.async_extraction | include_snippet(language) }}

{% endif %}
{% if features.plugin_system %}

## Plugin System

Kreuzberg supports extensible post-processing plugins for custom text transformation and filtering.

For detailed plugin documentation, visit [Plugin System Guide](https://kreuzberg.dev/guides/plugins/).

{% if snippets.plugin_system %}

### Plugin Example

{{ snippets.plugin_system | include_snippet(language) }}

{% endif %}
{% endif %}
{% if features.embeddings %}

## Embeddings Support

Generate vector embeddings for extracted text using the built-in ONNX Runtime support. Requires ONNX Runtime installation.

**[Embeddings Guide](https://kreuzberg.dev/features/#embeddings)**
{% endif %}

{% if snippets.batch_processing %}

## Batch Processing

Process multiple documents efficiently:

{{ snippets.batch_processing | include_snippet(language) }}

{% endif %}

## Configuration

For advanced configuration options including language detection, table extraction, OCR settings, and more:

**[Configuration Guide](https://kreuzberg.dev/guides/configuration/)**

## Documentation

- **[Official Documentation](https://kreuzberg.dev/)**
- **[API Reference](https://kreuzberg.dev/reference/api-{{ language }}/)**
- **[Examples & Guides](https://kreuzberg.dev/guides/)**

## Contributing

Contributions are welcome! See [Contributing Guide](https://github.com/kreuzberg-dev/kreuzberg/blob/main/CONTRIBUTING.md).

## License

MIT License - see LICENSE file for details.

## Support

- **Discord Community**: [Join our Discord](https://discord.gg/xt9WY3GnKR)
- **GitHub Issues**: [Report bugs](https://github.com/kreuzberg-dev/kreuzberg/issues)
- **Discussions**: [Ask questions](https://github.com/kreuzberg-dev/kreuzberg/discussions)
