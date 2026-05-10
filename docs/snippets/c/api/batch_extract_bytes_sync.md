```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    /* Items is a JSON array of BatchBytesItem objects.
     * Each entry has "content" (array of byte integers), "mime_type", and an optional "config". */
    const char *items_json =
        "["
        "  {\"content\": [72,101,108,108,111,33], \"mime_type\": \"text/plain\"},"
        "  {\"content\": [87,111,114,108,100,33], \"mime_type\": \"text/plain\"}"
        "]";

    KREUZBERGExtractionConfig *config = kreuzberg_extraction_config_default();

    /* Returns a JSON array of ExtractionResult objects, or NULL on failure. */
    char *results_json =
        kreuzberg_batch_extract_bytes_sync(items_json, config);
    if (!results_json) {
        fprintf(stderr, "batch extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        kreuzberg_extraction_config_free(config);
        return 1;
    }

    printf("%s\n", results_json);
    kreuzberg_free_string(results_json);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
