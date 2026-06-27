```java title="Element-Based Output (Java)"
import io.xberg.Xberg;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractionResult;
import io.xberg.Element;
import io.xberg.ResultFormat;
import java.nio.file.Path;
import java.util.List;

public class ElementBasedOutput {
    public static void main(String[] args) throws Exception {
        // Configure element-based output
        ExtractionConfig config = ExtractionConfig.builder()
            .withResultFormat(ResultFormat.ElementBased)
            .build();

        // Extract document
        ExtractionResult result = Xberg.extractSync(Path.of("document.pdf"), config);

        // Access elements
        List<Element> elements = result.elements();
        if (elements != null) {
            for (Element element : elements) {
                System.out.println("Type: " + element.elementType());

                String text = element.text();
                if (text.length() > 100) {
                    text = text.substring(0, 100);
                }
                System.out.println("Text: " + text);

                if (element.metadata().pageNumber() != null) {
                    System.out.println("Page: " + element.metadata().pageNumber());
                }

                if (element.metadata().coordinates() != null) {
                    System.out.println("Coords: " + element.metadata().coordinates());
                }

                System.out.println("---");
            }

            // Filter by element type
            elements.stream()
                .filter(e -> "Title".equalsIgnoreCase(String.valueOf(e.elementType())))
                .forEach(title -> {
                    String level = title.metadata().additional().getOrDefault("level", "unknown");
                    System.out.printf("[%s] %s%n", level, title.text());
                });
        }
    }
}
```
