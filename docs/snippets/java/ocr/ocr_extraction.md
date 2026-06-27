```java title="Java"
import io.xberg.ExtractInput;
import io.xberg.ExtractInputKind;
import io.xberg.ExtractedDocument;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractionResult;
import io.xberg.OcrConfig;
import io.xberg.Xberg;
import io.xberg.XbergRsException;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionConfig config = ExtractionConfig.builder()
                .withForceOcr(true)
                .withOcr(OcrConfig.builder()
                    .withBackend("tesseract")
                    .withLanguage(List.of("eng"))
                    .build())
                .build();

            ExtractInput input = ExtractInput.builder()
                .withKind(ExtractInputKind.Uri)
                .withUri("scanned.pdf")
                .build();

            ExtractionResult output = Xberg.extract(input, config);
            ExtractedDocument document = output.results().get(0);
            System.out.println(document.content());
        } catch (XbergRsException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```
