---
name: python-ocr-backend-integration
---
Integrate EasyOCR and PaddleOCR via Python

1. Wrap Python OCR class
2. Cache name and supported languages
3. For process_image():
   a. Clone Python object reference
   b. Spawn blocking task
   c. Acquire GIL inside task
   d. Call Python method
   e. Translate exceptions to Rust errors
   f. Return structured result
4. Handle timeouts and cancellation
