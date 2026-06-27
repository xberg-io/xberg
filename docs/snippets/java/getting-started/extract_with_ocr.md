```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;
import io.xberg.OcrConfig;
import java.io.IOException;

public class ExtractWithOCR {
    public static void main(String[] args) throws IOException {
        OcrConfig ocrConfig = OcrConfig.builder()
            .backend("tesseract")
            .language("eng")
            .build();

        ExtractionConfig config = ExtractionConfig.builder()
            .ocr(ocrConfig)
            .build();

        ExtractionResult result = Xberg.extract("scanned.pdf", config);

        System.out.println("Extracted text from scanned document:");
        System.out.println(result.getContent());
        System.out.println("Used OCR backend: tesseract");
    }
}
```
