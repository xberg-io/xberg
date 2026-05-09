```java title="Java"
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.EmbeddingBackendBridge;
import dev.kreuzberg.EmbeddingConfig;
import dev.kreuzberg.EmbeddingModelType;
import dev.kreuzberg.IEmbeddingBackend;
import dev.kreuzberg.KreuzbergRsException;
import java.util.ArrayList;
import java.util.List;

public class EmbeddingBackendExample {

    /**
     * Wrap an already-loaded embedder so kreuzberg can call back into it during
     * chunking and standalone embed requests.
     */
    static final class MyEmbedder implements IEmbeddingBackend {
        @Override
        public String name() {
            return "my-embedder";
        }

        @Override
        public String version() {
            return "1.0.0";
        }

        @Override
        public void initialize() {
            // Optional warm-up; runs once at registration before dimensions() is cached.
        }

        @Override
        public void shutdown() {
            // Optional cleanup.
        }

        @Override
        public long dimensions() {
            // Captured once at registration; the dispatcher uses this for shape validation.
            return 768L;
        }

        @Override
        public List<List<Float>> embed(List<String> texts) {
            // Delegate to the already-loaded host model.
            List<List<Float>> out = new ArrayList<>(texts.size());
            for (int i = 0; i < texts.size(); i++) {
                List<Float> row = new ArrayList<>(768);
                for (int j = 0; j < 768; j++) {
                    row.add(0.0f);
                }
                out.add(row);
            }
            return out;
        }
    }

    public static void main(String[] args) throws Exception {
        // Register once at startup.
        EmbeddingBackendBridge.registerEmbeddingBackend(new MyEmbedder());
        try {
            EmbeddingConfig config = EmbeddingConfig.builder()
                .model(new EmbeddingModelType.Plugin("my-embedder"))
                // Optional: bound the wait on a hung backend (default 60s; null disables).
                .maxEmbedDurationSecs(30L)
                .build();

            List<String> texts = List.of("Hello, world!", "Second text");
            List<List<Float>> vectors = Kreuzberg.embedTexts(texts, config);
            System.out.println("Generated " + vectors.size() + " vectors");
        } catch (KreuzbergRsException e) {
            e.printStackTrace();
        } finally {
            EmbeddingBackendBridge.unregisterEmbeddingBackend("my-embedder");
        }
    }
}
```
