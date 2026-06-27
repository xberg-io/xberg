```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractedDocument;
import io.xberg.XbergException;
import io.xberg.ExtractInput;
import io.xberg.ExtractionConfig;
import io.xberg.Table;
import java.io.IOException;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionResult output = Xberg.extract(
                ExtractInput.fromUri("document.pdf"),
                ExtractionConfig.builder().build()
            );
            ExtractedDocument result = output.results().get(0);

            for (Table table : result.getTables()) {
                System.out.println("Table with " + table.cells().size() + " rows");
                System.out.println(table.markdown());

                for (List<String> row : table.cells()) {
                    System.out.println(row);
                }
            }
        } catch (IOException | XbergException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```
