```c title="C"
#include <kreuzberg.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Shared state lives in `user_data` and is forwarded to every vtable callback.
 * Use atomics or a mutex if more than one thread can call into the plugin. */

typedef struct {
    atomic_size_t call_count;
} StatefulState;

static char *dup_cstr(const char *s) {
    size_t len = strlen(s);
    char *out = (char *)malloc(len + 1);
    if (out) {
        memcpy(out, s, len + 1);
    }
    return out;
}

static int32_t initialize_fn(const void *user_data, char **out_error) {
    (void)out_error;
    StatefulState *state = (StatefulState *)user_data;
    atomic_store(&state->call_count, 0);
    return 0;
}

static int32_t shutdown_fn(const void *user_data, char **out_error) {
    (void)out_error;
    const StatefulState *state = (const StatefulState *)user_data;
    size_t count = atomic_load(&state->call_count);
    fprintf(stderr, "stateful-plugin: shutdown after %zu calls\n", count);
    return 0;
}

static int32_t process_fn(
    const void *user_data,
    const char *result,
    const char *config,
    char **out_error
) {
    (void)result;
    (void)config;
    (void)out_error;
    StatefulState *state = (StatefulState *)user_data;
    atomic_fetch_add(&state->call_count, 1);
    return 0;
}

static int32_t processing_stage_fn(const void *user_data, char **out_result) {
    (void)user_data;
    *out_result = dup_cstr("\"Middle\"");
    return 0;
}

static int32_t priority_fn(const void *user_data) {
    (void)user_data;
    return 50;
}

static void free_user_data(void *user_data) {
    free(user_data);
}

int main(void) {
    StatefulState *state = (StatefulState *)malloc(sizeof(StatefulState));
    if (!state) {
        return 1;
    }
    atomic_init(&state->call_count, 0);

    KREUZBERGKreuzbergPostProcessorVTable vtable = {0};
    vtable.initialize_fn = initialize_fn;
    vtable.shutdown_fn = shutdown_fn;
    vtable.process = process_fn;
    vtable.processing_stage = processing_stage_fn;
    vtable.priority = priority_fn;
    vtable.free_user_data = free_user_data;

    char *err = NULL;
    int32_t rc = kreuzberg_register_post_processor(
        "stateful-plugin",
        vtable,
        state,
        &err
    );
    if (rc != 0) {
        fprintf(stderr, "register post-processor failed: %s\n",
                err ? err : "(no detail)");
        kreuzberg_free_string(err);
        free(state);
        return 1;
    }

    printf("stateful-plugin registered\n");
    return 0;
}
```
