```c title="C"
#include <kreuzberg.h>
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

static int32_t validate_fn(
    const void *user_data,
    const char *result,
    const char *config,
    char **out_error
) {
    (void)user_data;
    (void)config;

    /* Look for a "quality_score" key inside the metadata.additional map.
     * Production plugins should parse the JSON properly. */
    double score = 0.0;
    const char *needle = "\"quality_score\":";
    const char *found = strstr(result, needle);
    if (found) {
        score = atof(found + strlen(needle));
    }

    if (score < 0.5) {
        char buf[128];
        snprintf(buf, sizeof(buf),
                 "Quality score too low: %.2f < 0.50", score);
        *out_error = dup_cstr(buf);
        return 1;
    }
    return 0;
}

static int32_t priority_fn(const void *user_data) {
    (void)user_data;
    return 50;
}

int main(void) {
    KREUZBERGKreuzbergValidatorVTable vtable = {0};
    vtable.validate = validate_fn;
    vtable.priority = priority_fn;

    char *err = NULL;
    int32_t rc = kreuzberg_register_validator(
        "quality-score-validator",
        vtable,
        NULL,
        &err
    );
    if (rc != 0) {
        fprintf(stderr, "register validator failed: %s\n", err ? err : "(no detail)");
        kreuzberg_free_string(err);
        return 1;
    }

    printf("quality-score-validator registered\n");
    return 0;
}
```
