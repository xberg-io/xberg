```c title="C"
#include "kreuzberg.h"
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

    KREUZBERGExtractionConfig *config = kreuzberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    KREUZBERGExtractionResult *result =
        kreuzberg_extract_file_sync("paper.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        kreuzberg_extraction_config_free(config);
        return 1;
    }

    char *structured = kreuzberg_extraction_result_structured_output(result);
    if (structured) {
        printf("structured output (JSON):\n%s\n", structured);
        kreuzberg_free_string(structured);
    } else {
        printf("structured output: (none)\n");
    }

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```

<!-- snippet:syntax-only --> Requires network access to the configured LLM provider and a valid API key in the host environment.
