```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import java.io.IOException;

public class InstallVerify {
    public static void main(String[] args) throws IOException {
        System.out.println("Xberg FFI bindings loaded successfully");

        ExtractionResult result = Xberg.extract("sample.pdf");
        System.out.println("Installation verified!");
        System.out.println("Extracted " + result.getContent().length() + " characters");
    }
}
```
