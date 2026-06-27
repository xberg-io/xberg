```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractedDocument;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractInput;
import io.xberg.OcrConfig;

ExtractionConfig config = ExtractionConfig.builder()
    .ocr(OcrConfig.builder()
        .backend("tesseract")
        .build())
    .forceOcr(true)
    .build();

ExtractionResult output = Xberg.extract(
    ExtractInput.fromUri("document.pdf"),
    config
);

ExtractedDocument result = output.results().get(0);
System.out.println(result.getContent());
```
