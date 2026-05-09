```c title="C"
#include <kreuzberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Round-trip test: register a no-op validator, confirm it appears in the
 * registry list, then unregister and confirm it disappears. */

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
    (void)result;
    (void)config;
    (void)out_error;
    return 0;
}

static int32_t priority_fn(const void *user_data) {
    (void)user_data;
    return 50;
}

static int contains_name(const char *json, const char *name) {
    if (!json || !name) {
        return 0;
    }
    return strstr(json, name) != NULL;
}

int main(void) {
    const char *plugin_name = "noop-validator";

    KREUZBERGKreuzbergValidatorVTable vtable = {0};
    vtable.validate = validate_fn;
    vtable.priority = priority_fn;

    char *err = NULL;
    if (kreuzberg_register_validator(plugin_name, vtable, NULL, &err) != 0) {
        fprintf(stderr, "register failed: %s\n", err ? err : "(no detail)");
        kreuzberg_free_string(err);
        return 1;
    }

    char *list_after_register = kreuzberg_list_validators();
    if (!contains_name(list_after_register, plugin_name)) {
        fprintf(stderr, "FAIL: validator missing after register\n");
        kreuzberg_free_string(list_after_register);
        return 1;
    }
    printf("PASS: %s present after register\n", plugin_name);
    kreuzberg_free_string(list_after_register);

    if (kreuzberg_unregister_validator(plugin_name, &err) != 0) {
        fprintf(stderr, "unregister failed: %s\n", err ? err : "(no detail)");
        kreuzberg_free_string(err);
        return 1;
    }

    char *list_after_unregister = kreuzberg_list_validators();
    if (contains_name(list_after_unregister, plugin_name)) {
        fprintf(stderr, "FAIL: validator still present after unregister\n");
        kreuzberg_free_string(list_after_unregister);
        return 1;
    }
    printf("PASS: %s absent after unregister\n", plugin_name);
    kreuzberg_free_string(list_after_unregister);

    return 0;
}
```
