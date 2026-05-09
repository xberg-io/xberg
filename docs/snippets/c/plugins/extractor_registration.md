```c title="C"
#include <kreuzberg.h>
#include <stdio.h>

/*
 * The kreuzberg C FFI does not expose a public function for registering
 * custom DocumentExtractor implementations from C. Document extractors must
 * be registered from Rust via `kreuzberg::plugins::registry::get_document_extractor_registry()`
 * before the C library is loaded.
 *
 * From C you can inspect which extractors the core has registered:
 */

int main(void) {
    char *json = kreuzberg_list_document_extractors();
    if (!json) {
        fprintf(stderr, "list document extractors failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    printf("Registered document extractors: %s\n", json);
    kreuzberg_free_string(json);
    return 0;
}
```
