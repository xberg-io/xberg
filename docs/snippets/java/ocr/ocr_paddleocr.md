```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.XbergException;
import io.xberg.ExtractionConfig;
import io.xberg.OcrConfig;
import java.io.IOException;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionConfig config = ExtractionConfig.builder()
                .ocr(OcrConfig.builder()
                    .backend("paddle-ocr")
                    .language("en")
                    // .paddleOcrConfig(PaddleOcrConfig.builder().modelTier("server").build()) // for max accuracy
                    .build())
                .build();

            ExtractionResult result = Xberg.extract("scanned.pdf", config);
            System.out.println(result.getContent());
        } catch (IOException | XbergException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```
