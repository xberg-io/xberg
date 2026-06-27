```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractedDocument;
import io.xberg.ExtractInput;
import io.xberg.ExtractionConfig;
import java.io.IOException;
import java.util.Map;

public class ReadContent {
    public static void main(String[] args) throws IOException {
        ExtractionResult output = Xberg.extract(
            ExtractInput.fromUri("document.pdf"),
            ExtractionConfig.builder().build()
        );
        ExtractedDocument result = output.results().get(0);

        String content = result.getContent();
        var tables = result.getTables();
        var images = result.getImages();
        Map<String, Object> metadata = result.getMetadata();

        System.out.println("Content: " + content.length() + " characters");
        System.out.println("Tables: " + tables.size());
        System.out.println("Images: " + images.size());
        if (metadata != null) {
            System.out.println("Metadata keys: " + metadata.keySet());
        }
    }
}
```
