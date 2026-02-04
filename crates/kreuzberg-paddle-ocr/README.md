# kreuzberg-paddle-ocr

PaddleOCR via ONNX Runtime for Kreuzberg - high-performance text detection and recognition using PaddlePaddle's OCR models.

Based on the original [paddle-ocr-rs](https://github.com/mg-chao/paddle-ocr-rs) by [mg-chao](https://github.com/mg-chao), this vendored version includes improvements for Kreuzberg integration:

- **Workspace Dependency Alignment**: Uses Kreuzberg's workspace dependencies for consistency
- **Edition 2024**: Updated to Rust 2024 edition
- **ndarray Compatibility**: Aligned with Kreuzberg's ndarray version requirements
- **Integration**: Designed to work seamlessly with Kreuzberg's OCR backend system

## Features

- Text detection using DBNet (Differentiable Binarization)
- Text recognition using CRNN (Convolutional Recurrent Neural Network)
- Angle detection for rotated text
- Support for multiple languages via PaddleOCR models
- ONNX Runtime for efficient CPU inference

## Usage

This crate is used internally by Kreuzberg when the `paddle-ocr` feature is enabled:

```toml
[dependencies]
kreuzberg = { version = "4.2", features = ["paddle-ocr"] }
```

## Models

PaddleOCR models are automatically downloaded and cached on first use. Supported models include:

- PP-OCRv4 detection model
- PP-OCRv4 recognition model
- Mobile angle classification model

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgements

This project is based on the original [paddle-ocr-rs](https://github.com/mg-chao/paddle-ocr-rs) by [mg-chao](https://github.com/mg-chao), originally licensed under Apache-2.0. We are grateful for the foundational work that made this integration possible.

The original paddle-ocr-rs provides Rust bindings for PaddlePaddle's OCR models via ONNX Runtime, enabling efficient text detection and recognition without Python dependencies.
