# PDF Form Fields

Pull the filled values out of AcroForm/XFA PDFs as structured fields you can forward to downstream systems — extract checkboxes, text inputs, dropdowns, radio buttons, and signature fields directly without OCR.

## Overview

Fillable PDFs store form structure in two ways:

- **AcroForm** — Static form specification with field metadata. Fully supported.
- **XFA** — XML-based dynamic forms. Currently returns empty results (use AcroForm as workaround).

Extracted fields appear in `result.form_fields` as a list of `PdfFormField` structs, each carrying:

| Property | Type | Description |
|----------|------|-------------|
| `name` | string | Leaf field name in the hierarchy (e.g., `"line_total"`). |
| `full_name` | string | Dotted path from root (e.g., `"invoice.line_items[0].line_total"`). |
| `field_type` | enum | One of: `Text`, `Checkbox`, `Radio`, `Choice`, `Signature`, `Button`, `Unknown`. |
| `value` | string | Current field value (if filled). |
| `default_value` | string | Default value from the form template. |
| `flags` | u32 | Bitmask: read-only, required, multiline, password, and so on. |
| `page` | u32 | 1-indexed page number the field appears on. |
| `bbox` | `BoundingBox` | Widget location on the page (x, y, width, height). |
| `max_length` | u32 | Maximum input length (text fields only). |
| `tooltip` | string | Hover text or field description. |

## Configuration

Enable form field extraction (default):

```toml title="xberg.toml"
[pdf]
extract_form_fields = true
```

Disable (to skip form processing):

```toml
[pdf]
extract_form_fields = false
```

## Processing Form Fields

=== "Python"

    ```python
    from xberg import ExtractInput, extract, ExtractionConfig, PdfConfig

    config = ExtractionConfig(
        pdf_options=PdfConfig(extract_form_fields=True)
    )
    output = await extract(ExtractInput(kind="uri", uri="form.pdf"), config=config)
    result = output.results[0]

    for field in result.form_fields:
        print(f"Field: {field.full_name} = {field.value or '(empty)'}")
        print(f"  Type: {field.field_type}, Page: {field.page}")
    ```

=== "TypeScript"

    ```typescript
    import { ExtractInputKind, ExtractionConfig, extract } from "@xberg-io/xberg";

    const config: ExtractionConfig = {
      pdfOptions: { extractFormFields: true },
    };
    const output = await extract(
      { kind: ExtractInputKind.Uri, uri: "form.pdf" },
      config,
    );
    const result = output.results[0];

    for (const field of result.formFields ?? []) {
      console.log(`Field: ${field.fullName} = ${field.value || "(empty)"}`);
      console.log(`  Type: ${field.fieldType}, Page: ${field.page}`);
    }
    ```

=== "Rust"

    ```rust
    use xberg::{extract, ExtractInput, ExtractionConfig, PdfConfig};

    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            extract_form_fields: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let output = extract(ExtractInput::from_uri("form.pdf"), &config).await?;
    let result = &output.results[0];
    for field in &result.form_fields {
        let value = field.value.as_deref().unwrap_or("(empty)");
        println!("Field: {} = {}", field.full_name, value);
        println!("  Type: {:?}, Page: {:?}", field.field_type, field.page);
    }
    ```

=== "Go"

    ```go
    package main

    import (
        "fmt"
        xberg "github.com/xberg-io/xberg/packages/go"
    )

    func main() {
        extractFormFields := true
        config := xberg.ExtractionConfig{
            PdfOptions: &xberg.PdfConfig{
                ExtractFormFields: &extractFormFields,
            },
        }

        input := xberg.ExtractInputFromURI("form.pdf")
        output, err := xberg.Extract(*input, config)
        if err != nil {
            panic(err)
        }
        result := output.Results[0]

        for _, field := range result.FormFields {
            value := ""
            if field.Value != nil {
                value = *field.Value
            }
            fmt.Printf("Field: %s = %s\n", field.FullName, value)
            fmt.Printf("  Type: %v, Page: %v\n", field.FieldType, field.Page)
        }
    }
    ```

## Use Cases

**Form Auto-Fill**

Extract field values to populate templates or CRMs:

```python
output = await extract(ExtractInput(kind="uri", uri="invoice_form.pdf"))
result = output.results[0]
form_data = {f.full_name: f.value for f in result.form_fields if f.value}
# Submit form_data to downstream system
```

**Form Validation**

Check required fields and validate data before processing:

```python
required_fields = {f for f in result.form_fields if f.flags & 0x01}  # Check required bit
unfilled = {f.full_name for f in required_fields if not f.value}
if unfilled:
    print(f"Missing required fields: {unfilled}")
```

**Form-to-Data Conversion**

Convert fillable forms to structured JSON:

```python
form_json = {
    f.full_name: {
        "value": f.value,
        "type": f.field_type,
        "page": f.page,
        "bbox": f.bbox
    }
    for f in result.form_fields
}
```

## Limitations

- **XFA forms** — Dynamic XML-based forms are not yet supported; `form_fields` will be empty. Use AcroForm-based templates instead.
- **Flattened PDFs** — If form content is rendered into the PDF content stream (vs. stored as field metadata), the form structure is lost. Only editable/unfilled forms preserve field metadata.
- **Appearance streams** — Custom visual styling (button backgrounds, text colors) is not extracted; field values and types only.

## Best Practices

1. **Check field types** — Use `field_type` to handle different input types (text vs. checkbox vs. dropdown).
2. **Validate input** — Check `max_length` and format requirements before use.
3. **Preserve layout** — Use `bbox` and `page` to reconstruct form layout programmatically.
4. **Default values** — When a field has no `value`, consider using `default_value` as fallback.
5. **Test on real forms** — Form structures vary; test your extraction logic on representative PDFs from your sources.

## See also

- [PDF Extraction](extraction.md#pdf-extraction) — general PDF text extraction
- [Configuration Reference](../reference/configuration.md#pdfconfig) — all PDF options
