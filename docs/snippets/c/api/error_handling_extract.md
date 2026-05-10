```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    /* Mixed-validity batch: a real PDF, a missing file, and an unsupported type. */
    const char *items_json =
        "["
        "  {\"path\": \"document.pdf\"},"
        "  {\"path\": \"does-not-exist.pdf\"},"
        "  {\"path\": \"archive.unknownext\"}"
        "]";

    KREUZBERGExtractionConfig *config = kreuzberg_extraction_config_default();
    if (!config) {
        fprintf(stderr, "config init failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    /* Returns a JSON array of ExtractionResult objects (one per input, in order),
     * or NULL on a system-level failure. Per-item errors are encoded inside
     * each result object's metadata (e.g. an "errors" array). */
    char *results_json = kreuzberg_batch_extract_files(items_json, config);
    if (!results_json) {
        int32_t code = kreuzberg_last_error_code();
        const char *message = kreuzberg_last_error_context();
        /* message is valid until the next FFI call on this thread — copy if needed. */
        fprintf(stderr, "batch extraction aborted (code %d): %s\n",
                code, message ? message : "(no message)");
        kreuzberg_extraction_config_free(config);
        return code != 0 ? code : 1;
    }

    /* Walk the returned JSON. A real consumer would feed this to a JSON parser
     * and inspect each result's metadata.errors / content fields. */
    size_t len = strlen(results_json);
    printf("results (%zu bytes):\n%s\n", len, results_json);

    kreuzberg_free_string(results_json);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
