---
name: Smart Preprocessing
priority: high
---
Apply preprocessing conditionally based on image analysis

- Analyze image before preprocessing
- Upscale low-resolution images (< 150 DPI)
- Downsample extremely high-resolution images
- Apply noise reduction only if detected
- Apply contrast enhancement only if needed
- Skip preprocessing for quality images
