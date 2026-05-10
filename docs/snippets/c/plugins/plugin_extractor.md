```c title="C"
#include <kreuzberg.h>
#include <stdio.h>
#include <string.h>

/*
 * The C FFI exposes vtable-based registration for OCR backends, post-processors,
 * validators, and embedding backends. There is no public C entry point for
 * registering a custom DocumentExtractor — that must be done from Rust.
 *
 * From C you can still drive extraction for any MIME type the Rust core knows
 * how to handle. The example below feeds JSON bytes through the standard
 * extraction pipeline by passing the explicit MIME type.
 */

int main(void) {
    const char *json_payload = "{\"message\":\"Hello, world!\"}";
    const uint8_t *bytes = (const uint8_t *)json_payload;
    uintptr_t bytes_len = (uintptr_t)strlen(json_payload);

    KREUZBERGExtractionResult *result = kreuzberg_extract_bytes_sync(
        bytes,
        bytes_len,
        "application/json",
        NULL
    );

    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    char *content = kreuzberg_extraction_result_content(result);
    printf("Extracted JSON content: %s\n", content ? content : "(empty)");

    kreuzberg_free_string(content);
    kreuzberg_extraction_result_free(result);
    return 0;
}
```
