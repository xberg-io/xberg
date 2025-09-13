# DPI Configuration Implementation TODO

## Overview

Fix "Image too large" errors across all OCR backends by implementing comprehensive DPI configuration and automatic image size management.

## Problem Statement

- PDF pages converted to extremely tall images (38,749 pixels) causing Tesseract failures
- Hardcoded 200 DPI in pypdfium2 conversion creates oversized images
- No DPI normalization for direct image files
- Affects all OCR backends (Tesseract, EasyOCR, PaddleOCR)

## Implementation Plan

### Phase 1: Configuration Foundation

- [x] **Add DPI configuration fields to ExtractionConfig**
  - `target_dpi: int = 150` - Target DPI for processing
  - `max_image_dimension: int = 25000` - Max pixel dimension
  - `auto_adjust_dpi: bool = True` - Auto-adjust for size constraints
  - `min_dpi: int = 72` - Minimum DPI threshold
  - `max_dpi: int = 600` - Maximum DPI threshold

### Phase 2: Core Utilities

- [x] **Create `kreuzberg/_utils/_image_preprocessing.py`**
  - `normalize_image_dpi()` - Main normalization function
  - `calculate_optimal_dpi()` - DPI calculation based on constraints
  - `get_dpi_adjustment_heuristics()` - Smart adjustment logic

### Phase 3: Integration

- [x] **Update PDF extractor (`kreuzberg/_extractors/_pdf.py`)**
  - Integrate DPI normalization into `_convert_pdf_to_images()`
  - Replace hardcoded 200 DPI with configurable values
  - Add automatic scaling for large pages

- [x] **Update Image extractor (`kreuzberg/_extractors/_image.py`)**
  - Add image preprocessing before OCR
  - Handle oversized direct image files

### Phase 4: OCR Backend Updates

- [x] **OCR backends already compatible**
  - All backends accept normalized PIL Images from upstream extractors
  - No changes needed - cleaner architecture with preprocessing at source

### Phase 5: Testing & Validation

- [x] **Add comprehensive tests**
  - Test with sharable-web-guide.pdf (known to cause issues)
  - Test DPI configuration validation
  - Test auto-adjustment with different DPI settings
  - Validate all OCR backends work with normalized images

### Phase 6: Documentation

- [ ] **Update documentation**
  - Add DPI configuration examples to docs
  - Document performance vs quality tradeoffs
  - Add troubleshooting guide for image size issues

## Technical Details

### Configuration Schema

```python
@dataclass(unsafe_hash=True, slots=True)
class ExtractionConfig(ConfigDict):
    # ... existing fields ...
    target_dpi: int = 150
    max_image_dimension: int = 25000
    auto_adjust_dpi: bool = True
    min_dpi: int = 72
    max_dpi: int = 600
```

### Key Functions

- `normalize_image_dpi(image, config) -> (normalized_image, metadata)`
- `calculate_optimal_dpi(width, height, target_dpi, max_dim) -> optimal_dpi`
- `get_dpi_adjustment_heuristics(dimensions, content_type) -> adjustments`

### Integration Points

1. **PDF Processing**: Before creating PIL images from pypdfium2 pages
1. **Image Processing**: Before passing images to OCR backends
1. **OCR Backends**: Preprocessing step in all `process_image()` methods

## Success Criteria

- [ ] No more "Image too large" errors on any OCR backend
- [ ] Configurable DPI with sensible defaults
- [ ] Automatic scaling maintains OCR quality
- [ ] Backward compatibility preserved
- [ ] All existing tests pass
- [ ] New comprehensive tests for DPI functionality

## Current Status

- [x] Architecture analysis completed
- [x] Comprehensive solution designed
- [x] **Implementation completed!**
- [x] DPI configuration added to ExtractionConfig
- [x] Image preprocessing utilities created
- [x] PDF and Image extractors updated
- [x] Comprehensive tests added
- [ ] Ready for validation testing

---

## Last updated: 2025-09-13
