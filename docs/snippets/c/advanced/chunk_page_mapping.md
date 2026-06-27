```c title="C"
#include "xberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const char *config_json =
        "{"
        "\"chunking\": {"
        "\"chunker_type\": \"character\","
        "\"max_characters\": 500,"
        "\"overlap\": 50"
        "},"
        "\"pages\": {"
        "\"extract_pages\": true"
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

    char *chunks_json = xberg_extraction_result_chunks(result);
    printf("chunks (JSON, includes per-chunk first_page/last_page metadata):\n%s\n",
           chunks_json ? chunks_json : "[]");
    xberg_free_string(chunks_json);

    char *pages_json = xberg_extraction_result_pages(result);
    printf("pages (JSON): %s\n", pages_json ? pages_json : "[]");
    xberg_free_string(pages_json);

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
