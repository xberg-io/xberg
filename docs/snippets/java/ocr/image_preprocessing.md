```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.ImagePreprocessingConfig;
import dev.kreuzberg.OcrConfig;
import dev.kreuzberg.TesseractConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .tesseractConfig(TesseractConfig.builder()
            .preprocessing(ImagePreprocessingConfig.builder()
                .targetDpi(300)
                .denoise(true)
                .deskew(true)
                .contrastEnhance(true)
                .binarizationMethod("otsu")
                .build())
            .build())
        .build())
    .build();
```
