```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.OcrConfig;
import dev.kreuzberg.types.OcrElement;
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

            ExtractionResult result = Kreuzberg.extractFile("scanned.pdf", config);

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
        } catch (IOException | KreuzbergException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```
