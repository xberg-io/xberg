# QR-Code Detection

Decode QR codes embedded in extracted images. Detection runs over every `ExtractedImage` and populates `ExtractedImage.qr_codes` with the decoded payloads.

!!! Note "Feature gate"
    Requires the `qr-codes` Cargo feature. Included in `no-ort-target`, `wasm-target`, `android-target`, and `full`. Pure-Rust — no native deps, no ONNX, no LLM.

## When to Use

- You need to extract URLs, vCards, or payment data embedded as QR codes in scanned documents.
- You ingest forms, tickets, or invoices that embed QR codes for routing or audit.
- You need a network-free decoder that ships in every Xberg build (including WASM and Android).

## When Not to Use

- You only need standalone QR images, not embedded in larger documents. Use [rqrr](https://docs.rs/rqrr/) directly.
- You need barcodes other than QR. Xberg only decodes QR for now.
- You need barcode orientation correction. The decoder processes images as-is.

## Configuration

=== "Python"

    ```python title="Python"
    from xberg import extract, ExtractionConfig

    config = ExtractionConfig(qr_codes=True)
    result = await extract("ticket.pdf", config=config)
    for image in result.images or []:
        for qr in image.qr_codes or []:
            print(qr.payload)
    ```

=== "TypeScript"

    ```typescript title="TypeScript"
    import { extractFile } from '@xberg-io/xberg';

    const result = await extractFile("ticket.pdf", { qrCodes: true });
    for (const image of result.images ?? []) {
        for (const qr of image.qrCodes ?? []) {
            console.log(qr.payload);
        }
    }
    ```

=== "Rust"

    ```rust title="Rust"
    use xberg::{extract, ExtractionConfig};

    let config = ExtractionConfig {
        qr_codes: Some(true),
        ..Default::default()
    };
    let result = extract("ticket.pdf", None, &config).await?;
    for image in &result.images {
        if let Some(qrs) = &image.qr_codes {
            for qr in qrs {
                println!("{}", qr.payload);
            }
        }
    }
    ```

=== "TOML"

    ```toml title="xberg.toml"
    qr_codes = true
    ```

## Output Shape

`ExtractedImage.qr_codes` is `Option<Vec<QrCode>>`. JSON shape:

```json
{
  "images": [
    {
      "image_kind": "qr_code",
      "qr_codes": [
        {
          "payload": "https://example.com/order/4f2a",
          "confidence": 1.0,
          "bbox": { "x": 120, "y": 340, "width": 96, "height": 96 }
        }
      ]
    }
  ]
}
```

A single image can carry multiple QR codes — the decoder finds every QR in the image, not just the first.

### Field Reference

- `payload: String` — decoded text (URL, vCard, plain text, anything `rqrr` decodes)
- `confidence: Option<f32>` — always `Some(1.0)` for `rqrr` (successful decode = high confidence); `None` if detection is skipped
- `bbox: Option<QrBoundingBox>` — pixel-space top-left (`x`, `y`) and dimensions (`width`, `height`) when available

## Payload Encoding

`payload` is a `String`. The `rqrr` decoder decodes UTF-8 payloads directly. Non-UTF-8 payloads fall back to a best-effort lossy decode — characters that fail UTF-8 validation are replaced with the Unicode replacement character (U+FFFD). Use the byte-level decode path in `rqrr` directly if you need raw bytes.

## Availability

- Enabled by default when you set `qr_codes = true` in config.
- `None` when QR detection is disabled.
- `Some(vec![])` when detection ran but found no QR codes.
- Works across all bindings: Python, TypeScript, Rust, Ruby, Java, Go, Elixir, C#, PHP, Dart, Swift, Kotlin Android.

## Related

- [`ExtractedImage`](../reference/types.md) — type reference
- [VLM Image Captions](image-captions.md) — sibling per-image enrichment
- [Configuration Reference](../reference/configuration.md#extractionconfig) — config field reference
- [rqrr](https://docs.rs/rqrr/) — upstream pure-Rust decoder
