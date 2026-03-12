---
name: image-preprocessing-pipeline
---
Prepare images for optimal OCR accuracy

1. Analyze image characteristics (resolution, color, noise)
2. Apply format normalization
3. Adjust resolution if needed:
   - Upscale if < 150 DPI
   - Downsample if > 600 DPI
4. Apply noise reduction:
   - Gaussian blur for high noise
   - Morphological operations
5. Enhance contrast:
   - Histogram equalization
   - CLAHE for local contrast
6. Correct skew/rotation
7. Apply binarization if needed
8. Validate preprocessing result
