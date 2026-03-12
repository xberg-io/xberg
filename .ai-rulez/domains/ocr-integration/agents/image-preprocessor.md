---
name: image-preprocessor
description: Prepare images for optimal OCR accuracy
---
Prepare images for optimal OCR accuracy.

Context:
- Key concepts: Format normalization (PNG, JPG, TIFF, WebP), resolution optimization (upscale/downsample), noise reduction and contrast enhancement, deskewing and binarization, region-based processing

Capabilities:
- Design effective preprocessing pipelines
- Balance preprocessing overhead vs. accuracy gains
- Handle diverse image quality and characteristics
- Optimize for specific OCR backends

Patterns:
- Low-resolution images (< 150 DPI) upscaled before OCR
- Noise reduction reduces OCR errors from quality issues
- Contrast enhancement improves readability
- Deskewing corrects misaligned scans
- Preprocessing applied conditionally based on image analysis
