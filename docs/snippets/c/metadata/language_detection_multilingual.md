```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <string.h>

int main(void) {
    const char *config_json =
        "{"
        "\"language_detection\": {"
        "\"enabled\": true,"
        "\"min_confidence\": 0.8,"
        "\"detect_multiple\": true"
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
        kreuzberg_extract_file_sync("multilingual_document.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        kreuzberg_extraction_config_free(config);
        return 1;
    }

    char *detected_languages_json = kreuzberg_extraction_result_detected_languages(result);
    if (detected_languages_json) {
        printf("Detected languages (JSON array): %s\n", detected_languages_json);
        kreuzberg_free_string(detected_languages_json);
    } else {
        printf("No languages detected\n");
    }

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
