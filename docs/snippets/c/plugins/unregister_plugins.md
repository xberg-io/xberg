```c title="C"
#include <kreuzberg.h>
#include <stdio.h>

static int unregister_or_log(
    int32_t (*unregister_fn)(const char *, char **),
    const char *kind,
    const char *name
) {
    char *err = NULL;
    int32_t rc = unregister_fn(name, &err);
    if (rc != 0) {
        fprintf(stderr, "unregister %s '%s' failed: %s\n",
                kind,
                name,
                err ? err : "(no detail)");
        kreuzberg_free_string(err);
        return 1;
    }
    return 0;
}

int main(void) {
    int failures = 0;
    failures += unregister_or_log(kreuzberg_unregister_post_processor, "post-processor", "word-count");
    failures += unregister_or_log(kreuzberg_unregister_validator, "validator", "min-length-validator");
    failures += unregister_or_log(kreuzberg_unregister_ocr_backend, "OCR backend", "my-ocr");
    failures += unregister_or_log(kreuzberg_unregister_embedding_backend, "embedding backend", "my-embedder");
    return failures == 0 ? 0 : 1;
}
```
