```c title="C"
#include "xberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const char *config_json =
        "{"
        "\"enable_quality_processing\": true"
        "}";

    XBERGExtractionConfig *config = xberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        return 1;
    }

    XBERGExtractionResult *result =
        xberg_extract_sync("scanned_document.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        xberg_extraction_config_free(config);
        return 1;
    }

    double score = xberg_extraction_result_quality_score(result);
    if (score < 0.5) {
        printf("Warning: Low quality extraction (%.2f)\n", score);
    } else {
        printf("Quality score: %.2f\n", score);
    }

    char *warnings_json = xberg_extraction_result_processing_warnings(result);
    printf("processing warnings (JSON): %s\n", warnings_json ? warnings_json : "[]");
    xberg_free_string(warnings_json);

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
