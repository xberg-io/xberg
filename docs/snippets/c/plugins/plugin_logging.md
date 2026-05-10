```c title="C"
#include <kreuzberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Demonstrates structured logging from a post-processor plugin's lifecycle
 * hooks (initialize/shutdown) and from the per-result process callback. */

static char *dup_cstr(const char *s) {
    size_t len = strlen(s);
    char *out = (char *)malloc(len + 1);
    if (out) {
        memcpy(out, s, len + 1);
    }
    return out;
}

static int32_t initialize_fn(const void *user_data, char **out_error) {
    (void)user_data;
    (void)out_error;
    fprintf(stderr, "[INFO] plugin=logging-demo event=initialize\n");
    return 0;
}

static int32_t shutdown_fn(const void *user_data, char **out_error) {
    (void)user_data;
    (void)out_error;
    fprintf(stderr, "[INFO] plugin=logging-demo event=shutdown\n");
    return 0;
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

    size_t len = strlen(result);
    fprintf(stderr,
            "[INFO] plugin=logging-demo event=process bytes=%zu\n",
            len);

    if (strstr(result, "\"content\":\"\"") != NULL) {
        fprintf(stderr,
                "[WARN] plugin=logging-demo event=empty_content\n");
    }
    return 0;
}

static int32_t processing_stage_fn(const void *user_data, char **out_result) {
    (void)user_data;
    *out_result = dup_cstr("\"Late\"");
    return 0;
}

static int32_t priority_fn(const void *user_data) {
    (void)user_data;
    return 50;
}

int main(void) {
    KREUZBERGKreuzbergPostProcessorVTable vtable = {0};
    vtable.initialize_fn = initialize_fn;
    vtable.shutdown_fn = shutdown_fn;
    vtable.process = process_fn;
    vtable.processing_stage = processing_stage_fn;
    vtable.priority = priority_fn;

    char *err = NULL;
    int32_t rc = kreuzberg_register_post_processor(
        "logging-demo",
        vtable,
        NULL,
        &err
    );
    if (rc != 0) {
        fprintf(stderr, "[ERROR] register post-processor failed: %s\n",
                err ? err : "(no detail)");
        kreuzberg_free_string(err);
        return 1;
    }

    printf("logging-demo post-processor registered\n");
    return 0;
}
```
