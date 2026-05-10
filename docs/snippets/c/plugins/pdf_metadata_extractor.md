```c title="C"
#include <kreuzberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * The C FFI does not expose registration for custom DocumentExtractor
 * implementations. To add PDF-specific behaviour from C, register a
 * post-processor that runs only on PDF results and enriches them.
 *
 * The example below logs whenever the pipeline emits a PDF result, scoped
 * via the should_process hook so it never fires for other MIME types.
 */

static char *dup_cstr(const char *s) {
    size_t len = strlen(s);
    char *out = (char *)malloc(len + 1);
    if (out) {
        memcpy(out, s, len + 1);
    }
    return out;
}

static int32_t process_fn(
    const void *user_data,
    const char *result,
    const char *config,
    char **out_error
) {
    (void)user_data;
    (void)config;
    (void)out_error;
    printf("pdf-metadata-extractor: serialised PDF result is %zu bytes\n", strlen(result));
    return 0;
}

static int32_t processing_stage_fn(
    const void *user_data,
    char **out_result
) {
    (void)user_data;
    *out_result = dup_cstr("\"Late\"");
    return 0;
}

static int32_t should_process_fn(
    const void *user_data,
    const char *result,
    const char *config
) {
    (void)user_data;
    (void)config;
    return strstr(result, "\"mime_type\":\"application/pdf\"") != NULL;
}

static int32_t priority_fn(const void *user_data) {
    (void)user_data;
    return 75;
}

int main(void) {
    KREUZBERGKreuzbergPostProcessorVTable vtable = {0};
    vtable.process = process_fn;
    vtable.processing_stage = processing_stage_fn;
    vtable.should_process = should_process_fn;
    vtable.priority = priority_fn;

    char *err = NULL;
    int32_t rc = kreuzberg_register_post_processor(
        "pdf-metadata-extractor",
        vtable,
        NULL,
        &err
    );
    if (rc != 0) {
        fprintf(stderr, "register post-processor failed: %s\n", err ? err : "(no detail)");
        kreuzberg_free_string(err);
        return 1;
    }

    printf("pdf-metadata-extractor registered\n");
    return 0;
}
```
