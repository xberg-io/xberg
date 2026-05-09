```c title="C"
#include <kreuzberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * Minimal Validator skeleton: implements the required `validate` function
 * and the optional `priority` and `should_validate` hooks via the C vtable.
 */

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
    (void)user_data;
    (void)config;

    /* Reject results whose serialised form contains a clearly forbidden token. */
    if (strstr(result, "FORBIDDEN") != NULL) {
        *out_error = dup_cstr("Content contains forbidden token");
        return 1;
    }
    return 0;
}

static int32_t should_validate_fn(
    const void *user_data,
    const char *result,
    const char *config
) {
    (void)user_data;
    (void)result;
    (void)config;
    return 1;  /* always run */
}

static int32_t priority_fn(const void *user_data) {
    (void)user_data;
    return 50;
}

int main(void) {
    KREUZBERGKreuzbergValidatorVTable vtable = {0};
    vtable.validate = validate_fn;
    vtable.should_validate = should_validate_fn;
    vtable.priority = priority_fn;

    char *err = NULL;
    int32_t rc = kreuzberg_register_validator(
        "forbidden-token-validator",
        vtable,
        NULL,
        &err
    );
    if (rc != 0) {
        fprintf(stderr, "register validator failed: %s\n", err ? err : "(no detail)");
        kreuzberg_free_string(err);
        return 1;
    }

    printf("forbidden-token-validator registered\n");
    return 0;
}
```
