```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.XbergException;
import java.io.IOException;

public class CustomExtractorExample {
    public static void main(String[] args) {
        try {
            ExtractionResult result = Xberg.extract("document.json");
            System.out.println("Extracted content length: " + result.getContent().length());
        } catch (IOException | XbergException e) {
            e.printStackTrace();
        }
    }
}
```
