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
        kreuzberg_extract_file_sync("document.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        kreuzberg_extraction_config_free(config);
        return 1;
    }

    double score = kreuzberg_extraction_result_quality_score(result);
    printf("quality score: %.2f\n", score);

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
