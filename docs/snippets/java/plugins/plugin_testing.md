```java title="Java"
import io.xberg.ExtractedDocument;
import io.xberg.PostProcessor;
import org.junit.jupiter.api.Test;
import java.util.HashMap;
import java.util.Map;
import static org.junit.jupiter.api.Assertions.*;

class PostProcessorTest {
    @Test
    void testWordCountProcessor() {
        PostProcessor processor = result -> {
            long count = result.getContent().split("\\s+").length;

            Map<String, Object> metadata = new HashMap<>(result.getMetadata());
            metadata.put("word_count", count);

            return result;
        };

        ExtractedDocument input = new ExtractedDocument(
            "Hello world test",
            "text/plain",
            new HashMap<>(),
            java.util.List.of(),
            java.util.List.of(),
            java.util.List.of(),
            java.util.List.of(),
            true
        );

        ExtractedDocument output = processor.process(input);

        assertEquals(3, output.getMetadata().get("word_count"));
    }
}
```
