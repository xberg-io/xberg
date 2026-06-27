```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractedDocument;
import io.xberg.ExtractInput;
import io.xberg.ExtractionConfig;
import io.xberg.XbergException;
import java.io.IOException;

public class CustomExtractorExample {
    public static void main(String[] args) {
        try {
            ExtractionResult output = Xberg.extract(
                ExtractInput.fromUri("document.json"),
                ExtractionConfig.builder().build()
            );
            ExtractedDocument result = output.results().get(0);
            System.out.println("Extracted content length: " + result.getContent().length());
        } catch (IOException | XbergException e) {
            e.printStackTrace();
        }
    }
}
```
