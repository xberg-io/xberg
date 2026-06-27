```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionConfig;
import io.xberg.ExtractedDocument;
import io.xberg.HtmlOutputConfig;
import io.xberg.HtmlTheme;
import io.xberg.OutputFormat;
import java.nio.file.Path;
import java.util.Optional;

public class HtmlOutput {
    public static void main(String[] args) throws Exception {
        HtmlOutputConfig htmlOutput = HtmlOutputConfig.builder()
            .withTheme(HtmlTheme.GitHub)
            .withEmbedCss(true)
            .build();

        ExtractionConfig config = ExtractionConfig.builder()
            .withOutputFormat(OutputFormat.Html)
            .withHtmlOutput(Optional.of(htmlOutput))
            .build();

        var resultOutput = Xberg.extract(
            io.xberg.ExtractInput.builder()
                .withKind(io.xberg.ExtractInputKind.Uri)
                .withUri("document.pdf")
                .build(),
            config
        );
        ExtractedDocument result = resultOutput.results().get(0);
        System.out.println(result.content()); // HTML with kb-* classes
    }
}
```
