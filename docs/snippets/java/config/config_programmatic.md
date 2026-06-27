```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ChunkingConfig;
import io.xberg.ExtractionConfig;
import io.xberg.OcrConfig;
import io.xberg.TesseractConfig;

public final class ProgrammaticConfigExample {
    public static void main(String[] args) throws Exception {
        ExtractionConfig config = ExtractionConfig.builder()
            .ocr(OcrConfig.builder()
                .backend("tesseract")
                .language("eng+deu")
                .tesseractConfig(TesseractConfig.builder()
                    .psm(6)
                    .build())
                .build())
            .chunking(ChunkingConfig.builder()
                .maxChars(1000)
                .maxOverlap(200)
                .build())
            .useCache(true)
            .enableQualityProcessing(true)
            .build();

        ExtractionResult result = Xberg.extract("document.pdf", config);
        System.out.printf("Content length: %d%n", result.getContent().length());
    }
}
```
