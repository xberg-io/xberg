# HTML Output

Render extracted document content as styled HTML with semantic `kb-*` CSS classes, configurable themes, and full CSS customization. See the [HtmlOutputConfig reference](../reference/configuration.md#htmloutputconfig) for all options.

## Quick Start

=== "CLI"

    ```bash title="Terminal"
    xberg extract doc.pdf --html-theme github
    ```

=== "Python"

    ```python title="html_output.py"
    from xberg import ExtractInput, ExtractionConfig, HtmlOutputConfig, HtmlTheme, extract

    config = ExtractionConfig(
        output_format="html",
        html_output=HtmlOutputConfig(theme=HtmlTheme.GitHub),
    )
    output = await extract(ExtractInput(kind="uri", uri="doc.pdf"), config=config)
    result = output.results[0]
    print(result.content)  # styled HTML string
    ```

=== "TypeScript"

    ```typescript title="html_output.ts"
    import { ExtractInputKind, HtmlTheme, extract } from '@xberg-io/xberg';

    const output = await extract(
      { kind: ExtractInputKind.Uri, uri: 'doc.pdf' },
      {
        outputFormat: 'html',
        htmlOutput: { theme: HtmlTheme.GitHub },
      },
    );
    const result = output.results[0];
    console.log(result.content);
    ```

=== "Rust"

    ```rust title="html_output.rs"
    use xberg::{extract, ExtractInput, ExtractionConfig, HtmlOutputConfig, HtmlTheme};

    let config = ExtractionConfig {
        output_format: "html".to_string(),
        html_output: Some(HtmlOutputConfig {
            theme: HtmlTheme::GitHub,
            ..Default::default()
        }),
        ..Default::default()
    };
    let output = extract(ExtractInput::from_uri("doc.pdf"), &config).await?;
    let result = &output.results[0];
    println!("{}", result.content);
    ```

## Built-in Themes

| Theme                | Description                                                                            |
| -------------------- | -------------------------------------------------------------------------------------- |
| `unstyled` (default) | No built-in CSS. Only structural markup with `kb-*` classes. Use your own style sheet. |
| `default`            | System font stack, neutral colours, 72ch max width. All CSS custom properties defined. |
| `github`             | GitHub Markdown-inspired palette, border-bottom headings, 80ch max width.              |
| `dark`               | Dark background (#0d1117), light text. Good for terminal/IDE integrations.             |
| `light`              | Minimal light theme with generous spacing.                                             |

## Configuration

See [HtmlOutputConfig](../reference/configuration.md#htmloutputconfig) for detailed field documentation.

=== "Python"

    ```python title="html_config.py"
    from xberg import ExtractionConfig, HtmlOutputConfig, HtmlTheme

    config = ExtractionConfig(
        output_format="html",
        html_output=HtmlOutputConfig(
            theme=HtmlTheme.Dark,
            css="body { padding: 2rem; }",
            class_prefix="kb-",
            embed_css=True,
        ),
    )
    ```

=== "TypeScript"

    ```typescript title="html_config.ts"
    import { HtmlTheme } from '@xberg-io/xberg';

    const config = {
      outputFormat: 'html',
      htmlOutput: {
        theme: HtmlTheme.Dark,
        css: 'body { padding: 2rem; }',
        classPrefix: 'kb-',
        embedCss: true,
      },
    };
    ```

=== "Rust"

    ```rust title="html_config.rs"
    use xberg::{ExtractionConfig, HtmlOutputConfig, HtmlTheme};

    let config = ExtractionConfig {
        output_format: "html".to_string(),
        html_output: Some(HtmlOutputConfig {
            theme: HtmlTheme::Dark,
            css: Some("body { padding: 2rem; }".to_string()),
            class_prefix: "kb-".to_string(),
            embed_css: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

## CLI Flags

| Flag                           | Description                                                                                        |
| ------------------------------ | -------------------------------------------------------------------------------------------------- |
| `--html-theme <THEME>`         | Built-in theme: `default`, `github`, `dark`, `light`, `unstyled`. Implies `--content-format html`. |
| `--html-css <CSS>`             | Inline CSS string appended after the theme stylesheet.                                             |
| `--html-css-file <PATH>`       | Path to CSS file loaded at render time (max 1 MiB).                                                |
| `--html-class-prefix <PREFIX>` | CSS class prefix; default: `"kb-"`. Alphanumeric, hyphens, underscores only.                       |
| `--html-no-embed-css`          | Suppress the `<style>` block; use external stylesheet instead.                                     |

## CSS Customization

All built-in themes (except `unstyled`) define CSS custom properties on `:root`. Override them to adjust the theme without replacing it entirely:

```css title="custom.css"
:root {
  --kb-font-family: "Inter", sans-serif;
  --kb-text-color: #333;
  --kb-max-width: 60ch;
}
```

Pass custom CSS inline or from a file:

=== "CLI"

    ```bash title="Terminal"
    # Inline override
    xberg extract doc.pdf --html-theme github \
      --html-css ':root { --kb-max-width: 60ch; }'

    # From a file
    xberg extract doc.pdf --html-theme github \
      --html-css-file custom.css
    ```

=== "Python"

    ```python title="custom_css.py"
    from xberg import ExtractionConfig, HtmlOutputConfig, HtmlTheme

    config = ExtractionConfig(
        output_format="html",
        html_output=HtmlOutputConfig(
            theme=HtmlTheme.GitHub,
            css_file="custom.css",
        ),
    )
    ```

=== "Rust"

    ```rust title="custom_css.rs"
    use xberg::{ExtractionConfig, HtmlOutputConfig, HtmlTheme};
    use std::path::PathBuf;

    let config = ExtractionConfig {
        output_format: "html".to_string(),
        html_output: Some(HtmlOutputConfig {
            theme: HtmlTheme::GitHub,
            css_file: Some(PathBuf::from("custom.css")),
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

To use your own style sheet, set the theme to `unstyled` and disable the embedded `<style>` block:

```python title="external_stylesheet.py"
config = ExtractionConfig(
    output_format="html",
    html_output=HtmlOutputConfig(
        theme=HtmlTheme.Unstyled,
        embed_css=False,
    ),
)
```

## Class Reference

All generated HTML elements include semantic `kb-*` classes for targeted styling.

| Class                       | Element                | Description                   |
| --------------------------- | ---------------------- | ----------------------------- |
| `kb-doc`                    | `<div>`                | Root wrapper                  |
| `kb-content`                | `<main>`               | Content area                  |
| `kb-doc-title`              | `<h1>`                 | Document title                |
| `kb-h`, `kb-h1`..`kb-h6`    | `<h1>`..`<h6>`         | Headings                      |
| `kb-p`                      | `<p>`                  | Paragraphs                    |
| `kb-list`, `kb-ul`, `kb-ol` | `<ul>`, `<ol>`         | Lists                         |
| `kb-li`                     | `<li>`                 | List items                    |
| `kb-blockquote`             | `<blockquote>`         | Block quotes                  |
| `kb-pre`                    | `<pre>`                | Code blocks                   |
| `kb-code`                   | `<code>`               | Inline/block code             |
| `kb-table`                  | `<table>`              | Tables                        |
| `kb-thead`, `kb-tbody`      | `<thead>`, `<tbody>`   | Table sections                |
| `kb-th`, `kb-td`, `kb-tr`   | `<th>`, `<td>`, `<tr>` | Table cells/rows              |
| `kb-figure`                 | `<figure>`             | Image wrapper                 |
| `kb-img`                    | `<img>`                | Images                        |
| `kb-page-break`             | `<hr>`                 | Page breaks                   |
| `kb-footnote`               | `<aside>`              | Footnote definitions          |
| `kb-footnote-ref`           | `<sup>`                | Footnote references           |
| `kb-citation`               | `<cite>`               | Citations                     |
| `kb-link`                   | `<a>`                  | Hyperlinks                    |
| `kb-metadata`               | `<dl>`                 | Metadata blocks               |
| `kb-formula`                | `<pre>`                | Math formulas                 |
| `kb-slide`                  | `<section>`            | Slide sections                |
| `kb-dt`, `kb-dd`            | `<dt>`, `<dd>`         | Definition terms/descriptions |
| `kb-admonition`             | `<aside>`              | Admonitions                   |
| `kb-group`                  | `<div>`                | Grouped content               |

!!! Tip "Custom prefix" If you set `class_prefix` to `"my-"`, all classes become `my-doc`, `my-content`, `my-h1`, and so on.

## Security

!!! Warning "Security considerations" - `class_prefix` is validated to prevent HTML injection - `</style>` sequences are stripped from user CSS - `css_file` is limited to 1 MiB - When serving HTML to untrusted users, sanitize CSS at the application layer

## See Also

- [Configuration](configuration.md) -- all configuration options
- [Extraction Basics](extraction.md) -- core extraction API and supported formats
- [Element-Based Output](output-formats.md#element-based-output) -- structured element output as an alternative to HTML
- [Document Structure](output-formats.md#document-structure) -- how Xberg models document structure
