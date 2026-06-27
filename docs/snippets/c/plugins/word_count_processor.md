```c title="C"
#include <xberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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

    /* The `result` JSON string is read-only at this layer; for a real
     * mutating post-processor, decode the JSON, mutate, and serialise back
     * via the xberg ExtractedDocument helpers in your host language. */
    size_t words = 0;
    int in_word = 0;
    for (const char *p = result; *p; ++p) {
        if (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r') {
            in_word = 0;
        } else if (!in_word) {
            in_word = 1;
            words += 1;
        }
    }
    printf("word-count: ~%zu tokens in serialised result\n", words);
    return 0;
}

static int32_t processing_stage_fn(
    const void *user_data,
    char **out_result
) {
    (void)user_data;
    /* ProcessingStage is JSON-serialised; "Early" maps to ProcessingStage::Early. */
    *out_result = dup_cstr("\"Early\"");
    return 0;
}

static int32_t should_process_fn(
    const void *user_data,
    const char *result,
    const char *config
) {
    (void)user_data;
    (void)config;
    /* Skip empty content. */
    return strstr(result, "\"content\":\"\"") == NULL;
}

static int32_t priority_fn(const void *user_data) {
    (void)user_data;
    return 50;
}

int main(void) {
    XBERGXbergPostProcessorVTable vtable = {0};
    vtable.process = process_fn;
    vtable.processing_stage = processing_stage_fn;
    vtable.should_process = should_process_fn;
    vtable.priority = priority_fn;

    char *err = NULL;
    int32_t rc = xberg_register_post_processor(
        "word-count",
        vtable,
        NULL,
        &err
    );
    if (rc != 0) {
        fprintf(stderr, "register post-processor failed: %s\n", err ? err : "(no detail)");
        xberg_free_string(err);
        return 1;
    }

    printf("word-count post-processor registered\n");
    return 0;
}
```
