```c title="C"
#include "xberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const char *config_json =
        "{"
        "\"structured_extraction\": {"
        "\"schema\": {"
        "\"type\": \"object\","
        "\"properties\": {"
        "\"title\": {\"type\": \"string\"},"
        "\"authors\": {\"type\": \"array\", \"items\": {\"type\": \"string\"}},"
        "\"date\": {\"type\": \"string\"}"
        "},"
        "\"required\": [\"title\", \"authors\", \"date\"],"
        "\"additionalProperties\": false"
        "},"
        "\"llm\": {\"model\": \"openai/gpt-4o-mini\"},"
        "\"strict\": true"
        "}"
        "}";

    XBERGExtractionConfig *config = xberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        return 1;
    }

    XBERGExtractionResult *result =
        xberg_extract_sync("paper.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        xberg_extraction_config_free(config);
        return 1;
    }

    char *structured = xberg_extraction_result_structured_output(result);
    if (structured) {
        printf("structured output (JSON):\n%s\n", structured);
        xberg_free_string(structured);
    } else {
        printf("structured output: (none)\n");
    }

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```

<!-- snippet:syntax-only --> Requires network access to the configured LLM provider and a valid API key in the host environment.
