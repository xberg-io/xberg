```java title="Java"
import io.xberg.Xberg;
import io.xberg.ExtractionResult;
import io.xberg.ExtractionConfig;
import io.xberg.KeywordConfig;
import io.xberg.KeywordAlgorithm;
import java.util.List;
import java.util.Map;

ExtractionConfig config = ExtractionConfig.builder()
    .keywords(KeywordConfig.builder()
        .algorithm(KeywordAlgorithm.YAKE)
        .maxKeywords(10)
        .minScore(0.3)
        .build())
    .build();

ExtractionResult result = Xberg.extract("research_paper.pdf", config);

Map<String, Object> metadata = result.getMetadata() != null ? result.getMetadata() : Map.of();

if (metadata.containsKey("keywords")) {
    List<Map<String, Object>> keywords = (List<Map<String, Object>>) metadata.get("keywords");
    for (Map<String, Object> kw : keywords) {
        String text = (String) kw.get("text");
        Double score = ((Number) kw.get("score")).doubleValue();
        System.out.println(text + ": " + String.format("%.3f", score));
    }
}
```
