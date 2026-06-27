```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.XbergException;
import io.xberg.Table;
import java.io.IOException;
import java.util.List;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionResult result = Xberg.extract("document.pdf");

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
