```java title="Element-Based Output (Java)"
import io.xberg.Xberg;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractedDocument;
import io.xberg.Element;
import io.xberg.OutputFormat;

public class ElementBasedOutput {
    public static void main(String[] args) {
        // Configure element-based output
        ExtractionConfig config = new ExtractionConfig();
        config.setOutputFormat(OutputFormat.ELEMENT_BASED);

        // Extract document
        var resultOutput = Xberg.extract(
            io.xberg.ExtractInput.builder()
                .withKind(io.xberg.ExtractInputKind.Uri)
                .withUri("document.pdf")
                .build(),
            config
        );
        ExtractedDocument result = resultOutput.results().get(0);

        // Access elements
        for (Element element : result.getElements()) {
            System.out.println("Type: " + element.getElementType());

            String text = element.getText();
            if (text.length() > 100) {
                text = text.substring(0, 100);
            }
            System.out.println("Text: " + text);

            if (element.getMetadata().getPageNumber() != null) {
                System.out.println("Page: " + element.getMetadata().getPageNumber());
            }

            if (element.getMetadata().getCoordinates() != null) {
                var coords = element.getMetadata().getCoordinates();
                System.out.printf("Coords: (%f, %f) - (%f, %f)%n",
                    coords.getLeft(), coords.getTop(),
                    coords.getRight(), coords.getBottom());
            }

            System.out.println("---");
        }

        // Filter by element type
        result.getElements().stream()
            .filter(e -> "title".equals(e.getElementType()))
            .forEach(title -> {
                String level = (String) title.getMetadata()
                    .getAdditional()
                    .getOrDefault("level", "unknown");
                System.out.printf("[%s] %s%n", level, title.getText());
            });
    }
}
```
