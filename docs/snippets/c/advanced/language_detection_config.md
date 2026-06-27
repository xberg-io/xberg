```c title="C"
#include "xberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const char *config_json =
        "{"
        "\"language_detection\": {"
        "\"enabled\": true,"
        "\"min_confidence\": 0.8,"
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

    char *langs_json = xberg_extraction_result_detected_languages(result);
    printf("detected languages (JSON): %s\n", langs_json ? langs_json : "[]");
    xberg_free_string(langs_json);

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
