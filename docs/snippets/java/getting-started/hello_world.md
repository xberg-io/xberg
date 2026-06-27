```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import java.io.IOException;

public class HelloWorld {
    public static void main(String[] args) throws IOException {
        ExtractionResult result = Xberg.extract("document.pdf");
        System.out.println("Extracted content:");
        System.out.println(result.getContent().substring(0, Math.min(200, result.getContent().length())));
    }
}
```
