```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.OcrConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .language("eng+deu+fra")
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("multilingual.pdf", config);
System.out.println(result.getContent());
```
