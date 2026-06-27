```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;
import io.xberg.OcrConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .language("eng+deu+fra")
        .build())
    .build();

ExtractionResult result = Xberg.extract("multilingual.pdf", config);
System.out.println(result.getContent());
```
