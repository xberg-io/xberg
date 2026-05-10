```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const char *config_json =
        "{"
        "\"enable_quality_processing\": true"
        "}";

    KREUZBERGExtractionConfig *config = kreuzberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    KREUZBERGExtractionResult *result =
        kreuzberg_extract_file_sync("scanned_document.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        kreuzberg_extraction_config_free(config);
        return 1;
    }

    double score = kreuzberg_extraction_result_quality_score(result);
    if (score < 0.5) {
        printf("Warning: low quality extraction (%.2f)\n", score);
    } else {
        printf("Quality score: %.2f\n", score);
    }

    char *warnings_json = kreuzberg_extraction_result_processing_warnings(result);
    printf("processing warnings (JSON): %s\n", warnings_json ? warnings_json : "[]");
    kreuzberg_free_string(warnings_json);

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
