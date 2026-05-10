```c title="C"
#include <kreuzberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* user_data carries the minimum length threshold. */
typedef struct {
    size_t min_length;
} MinLengthState;

static char *dup_cstr(const char *s) {
    size_t len = strlen(s);
    char *out = (char *)malloc(len + 1);
    if (out) {
        memcpy(out, s, len + 1);
    }
    return out;
}

static int32_t validate_fn(
    const void *user_data,
    const char *result,
    const char *config,
    char **out_error
) {
    (void)config;
    const MinLengthState *state = (const MinLengthState *)user_data;

    /* `result` is a JSON string of ExtractionResult. We approximate the content
     * length check by scanning for the "content" field. Production plugins
     * should parse JSON properly. */
    const char *content = strstr(result, "\"content\":\"");
    size_t content_len = 0;
    if (content) {
        content += strlen("\"content\":\"");
        const char *end = strchr(content, '"');
        if (end) {
            content_len = (size_t)(end - content);
        }
    }

    if (content_len < state->min_length) {
        char buf[128];
        snprintf(buf, sizeof(buf),
                 "Content too short: %zu < %zu characters",
                 content_len,
                 state->min_length);
        *out_error = dup_cstr(buf);
        return 1;
    }
    return 0;
}

static int32_t priority_fn(const void *user_data) {
    (void)user_data;
    return 100;
}

static void free_user_data(void *user_data) {
    free(user_data);
}

int main(void) {
    MinLengthState *state = (MinLengthState *)malloc(sizeof(MinLengthState));
    state->min_length = 100;

    KREUZBERGKreuzbergValidatorVTable vtable = {0};
    vtable.validate = validate_fn;
    vtable.priority = priority_fn;
    vtable.free_user_data = free_user_data;

    char *err = NULL;
    int32_t rc = kreuzberg_register_validator(
        "min-length-validator",
        vtable,
        state,
        &err
    );
    if (rc != 0) {
        fprintf(stderr, "register validator failed: %s\n", err ? err : "(no detail)");
        kreuzberg_free_string(err);
        free(state);
        return 1;
    }

    printf("min-length-validator registered\n");
    return 0;
}
```
