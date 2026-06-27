```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractedDocument;
import io.xberg.XbergException;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractInput;
import io.xberg.OcrConfig;
import io.xberg.types.OcrElement;
import java.io.IOException;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionConfig config = ExtractionConfig.builder()
                .ocr(OcrConfig.builder()
                    .backend("paddle-ocr")
                    .language("en")
                    .build())
                .build();

            ExtractionResult output = Xberg.extract(
                ExtractInput.fromUri("scanned.pdf"),
                config
            );

            ExtractedDocument result = output.results().get(0);

            if (result.getOcrElements() != null) {
                for (OcrElement element : result.getOcrElements()) {
                    System.out.printf("Text: %s%n", element.getText());
                    System.out.printf("Confidence: %.2f%n", element.getConfidence().getRecognition());
                    System.out.printf("Geometry: %s%n", element.getGeometry());
                    if (element.getRotation() != null) {
                        System.out.printf("Rotation: %.1f°%n", element.getRotation().getAngle());
                    }
                    System.out.println();
                }
            }
        } catch (IOException | XbergException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```
