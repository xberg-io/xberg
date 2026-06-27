```c title="C"
#include "xberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const char *config_json =
        "{"
        "\"keywords\": {"
        "\"algorithm\": \"yake\","
        "\"max_keywords\": 10,"
        "\"min_score\": 0.3"
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
        xberg_extract_sync("research_paper.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        xberg_extraction_config_free(config);
        return 1;
    }

    char *keywords_json = xberg_extraction_result_extracted_keywords(result);
    if (keywords_json) {
        printf("Keywords: %s\n", keywords_json);
        xberg_free_string(keywords_json);
    } else {
        printf("Keywords: (none)\n");
    }

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
