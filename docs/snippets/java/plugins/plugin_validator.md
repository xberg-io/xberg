```java title="Java"
import dev.kreuzberg.IValidator;
import dev.kreuzberg.ExtractionConfig;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.ValidatorBridge;

// Generic validator pattern: every IValidator has the same shape.
// name() keys the registry, priority() orders execution (higher = earlier),
// should_validate() is a fast skip-check, and validate() throws on failure.
public class GenericValidator implements IValidator {
    private final String pluginName;
    private final int pluginPriority;

    public GenericValidator(String pluginName, int pluginPriority) {
        this.pluginName = pluginName;
        this.pluginPriority = pluginPriority;
    }

    @Override
    public String name() {
        return pluginName;
    }

    @Override
    public String version() {
        return "1.0.0";
    }

    @Override
    public void initialize() {
        // Optional: open resources, load config files, etc.
    }

    @Override
    public void shutdown() {
        // Optional: release resources held in initialize().
    }

    @Override
    public void validate(ExtractionResult result, ExtractionConfig config) throws Exception {
        if (result.content() == null || result.content().isBlank()) {
            throw new IllegalArgumentException("Extracted content is blank");
        }
    }

    @Override
    public boolean should_validate(ExtractionResult _result, ExtractionConfig _config) {
        return true;
    }

    @Override
    public int priority() {
        return pluginPriority;
    }

    public static void registerGenericValidator() {
        GenericValidator validator = new GenericValidator("non-empty-content", 200);
        ValidatorBridge.registerValidator(validator);
    }
}
```
