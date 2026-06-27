```c title="C"
#include "xberg.h"
#include <stdio.h>
#include <string.h>

int main(void) {
    const char *config_json =
        "{"
        "\"language_detection\": {"
        "\"enabled\": true,"
        "\"min_confidence\": 0.9,"
        "\"detect_multiple\": false"
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
        xberg_extract_sync("document.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        xberg_extraction_config_free(config);
        return 1;
    }

    XBERGMetadata *metadata = xberg_extraction_result_metadata(result);
    if (metadata) {
        char *language = xberg_metadata_language(metadata);
        if (language) {
            printf("Metadata language: %s\n", language);
            xberg_free_string(language);
        }
        xberg_metadata_free(metadata);
    }

    char *detected_languages_json = xberg_extraction_result_detected_languages(result);
    if (detected_languages_json) {
        printf("Detected languages: %s\n", detected_languages_json);
        xberg_free_string(detected_languages_json);
    } else {
        printf("No languages detected\n");
    }

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
