```java title="Java"
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.OcrConfig;
import dev.kreuzberg.TesseractConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .language("eng+fra")
        .tesseractConfig(TesseractConfig.builder()
            .psm(3)
            .build())
        .build())
    .build();
```
