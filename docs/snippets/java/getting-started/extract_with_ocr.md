```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.OcrConfig;
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

        ExtractionResult result = Kreuzberg.extractFile("scanned.pdf", config);

        System.out.println("Extracted text from scanned document:");
        System.out.println(result.getContent());
        System.out.println("Used OCR backend: tesseract");
    }
}
```
