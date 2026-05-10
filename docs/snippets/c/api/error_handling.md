```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    KREUZBERGExtractionConfig *config = kreuzberg_extraction_config_default();

    /* Pass an unsupported MIME type to trigger an error. */
    KREUZBERGExtractionResult *result =
        kreuzberg_extract_bytes_sync(NULL, 0, "application/x-unknown", config);
    if (!result) {
        int32_t code = kreuzberg_last_error_code();
        const char *message = kreuzberg_last_error_context();
        /* message is valid until the next FFI call on this thread — copy if needed. */
        fprintf(stderr, "error %d: %s\n", code, message ? message : "(no message)");
        kreuzberg_extraction_config_free(config);
        return code != 0 ? code : 1;
    }

    char *content = kreuzberg_extraction_result_content(result);
    printf("%s\n", content ? content : "(empty)");
    kreuzberg_free_string(content);

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
