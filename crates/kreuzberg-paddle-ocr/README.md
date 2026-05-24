# kreuzberg-paddle-ocr

[![Bindings](https://img.shields.io/badge/Bindings-alef%20%D7%90-007ec6)](https://github.com/kreuzberg-dev/alef)

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

## ONNX Runtime Requirement

This crate requires **ONNX Runtime 1.24+** at runtime.

Install it:

- **macOS (Homebrew)**: `brew install onnxruntime`
- **Linux**: Download from [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases)
- **Windows**: Download from [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases)

## Usage

This crate is used internally by Kreuzberg when the `paddle-ocr` feature is enabled:

```toml
[dependencies]
kreuzberg = { version = "4.2", features = ["paddle-ocr"] }
```

## Models

PaddleOCR models are automatically downloaded and cached on first use. Supported models include:

- PP-OCRv5 server detection model
- PP-OCRv5 per-family recognition models (11 script families)
- PPOCRv2 mobile angle classification model

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgements

This project is based on the original [paddle-ocr-rs](https://github.com/mg-chao/paddle-ocr-rs) by [mg-chao](https://github.com/mg-chao), originally licensed under Apache-2.0. We are grateful for the foundational work that made this integration possible.

The original paddle-ocr-rs provides Rust bindings for PaddlePaddle's OCR models via ONNX Runtime, enabling efficient text detection and recognition without Python dependencies.
