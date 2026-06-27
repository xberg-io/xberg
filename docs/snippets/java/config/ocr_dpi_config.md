```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;
import io.xberg.OcrConfig;
import io.xberg.ImagePreprocessingConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .build())
    .imagePreprocessing(ImagePreprocessingConfig.builder()
        .targetDpi(300)
        .build())
    .build();

ExtractionResult result = Xberg.extract("scanned.pdf", config);
```
