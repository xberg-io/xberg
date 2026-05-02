# HTML Styling Contract

This document defines the stability guarantees for Kreuzberg's styled HTML output.

## Stability Guarantee

All CSS class names emitted by the styled HTML renderer (prefixed by the configured `class_prefix`, default `kb-`) and all CSS custom properties (`--kb-*`) are **stable across minor versions**.

Breaking changes to class names or custom properties will only occur in major version bumps and will be documented in the changelog.

## CSS Custom Properties

All built-in themes define the following CSS custom properties on `:root`:

| Property                 | Default (default theme)   | Description            |
| ------------------------ | ------------------------- | ---------------------- |
| `--kb-font-family`       | `system-ui, sans-serif`   | Body font stack        |
| `--kb-mono-font-family`  | `ui-monospace, monospace` | Code font stack        |
| `--kb-text-color`        | `#1a1a1a`                 | Body text colour       |
| `--kb-bg-color`          | `#ffffff`                 | Background colour      |
| `--kb-heading-color`     | `#111111`                 | Heading colour         |
| `--kb-link-color`        | `#0066cc`                 | Link colour            |
| `--kb-link-hover-color`  | `#004499`                 | Link hover colour      |
| `--kb-code-bg`           | `#f5f5f5`                 | Code block background  |
| `--kb-code-color`        | `#c7254e`                 | Inline code colour     |
| `--kb-border-color`      | `#e0e0e0`                 | General border colour  |
| `--kb-table-border`      | `#cccccc`                 | Table border colour    |
| `--kb-blockquote-border` | `#0066cc`                 | Blockquote left border |
| `--kb-max-width`         | `72ch`                    | Content max width      |
| `--kb-line-height`       | `1.6`                     | Body line height       |

## Class Names

See the [HTML Output guide](../guides/html-output.md#class-reference) for the complete class reference.

## Versioning

- **Minor versions** (for example, 4.8 → 4.9): Classes and custom properties are additive only. No removals or renames.
- **Major versions** (for example, 4.x → 5.x): May remove or rename classes. All changes documented in migration guide.
- **Theme CSS**: Visual appearance (colours, spacing, fonts) may change in minor versions. Only structural class names are covered by the stability guarantee.
