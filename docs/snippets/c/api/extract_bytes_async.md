```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* kreuzberg_extract_bytes schedules work on the global Tokio runtime and
 * returns once extraction is complete.  For true non-blocking use, call it
 * from a dedicated OS thread and synchronize via a semaphore or callback. */
int main(void) {
    const char *text = "Hello, kreuzberg!";
    const uint8_t *bytes = (const uint8_t *)text;
    size_t len = strlen(text);

    KREUZBERGExtractionConfig *config = kreuzberg_extraction_config_default();

    KREUZBERGExtractionResult *result =
        kreuzberg_extract_bytes(bytes, len, "text/plain", config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        kreuzberg_extraction_config_free(config);
        return 1;
    }

    char *content = kreuzberg_extraction_result_content(result);
    printf("%s\n", content ? content : "(empty)");
    kreuzberg_free_string(content);

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
