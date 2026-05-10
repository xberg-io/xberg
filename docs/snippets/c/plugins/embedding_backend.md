```c title="C"
#include <kreuzberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Fixed embedding dimension produced by this backend. */
#define EMBED_DIM 768

static char *dup_cstr(const char *s) {
    size_t len = strlen(s);
    char *out = (char *)malloc(len + 1);
    if (out) {
        memcpy(out, s, len + 1);
    }
    return out;
}

static uintptr_t dimensions_fn(const void *user_data) {
    (void)user_data;
    return (uintptr_t)EMBED_DIM;
}

static int32_t embed_fn(
    const void *user_data,
    const char *texts,
    char **out_result,
    char **out_error
) {
    (void)user_data;
    (void)out_error;

    /* `texts` is a JSON array of strings. Count entries by scanning quotes;
     * a real backend would parse the JSON and call its host model. */
    size_t count = 0;
    int in_string = 0;
    int escape = 0;
    for (const char *p = texts; *p; ++p) {
        if (escape) {
            escape = 0;
        } else if (*p == '\\') {
            escape = 1;
        } else if (*p == '"') {
            if (!in_string) {
                in_string = 1;
                count += 1;
            } else {
                in_string = 0;
            }
        }
    }

    /* Build a JSON array of zero vectors of length EMBED_DIM, one per input. */
    /* Worst case bytes per entry: 2 brackets + EMBED_DIM * 4 ("0.0,") + comma. */
    size_t cap = 16 + count * (EMBED_DIM * 4 + 4);
    char *json = (char *)malloc(cap);
    if (!json) {
        *out_error = dup_cstr("allocation failure");
        return 1;
    }
    size_t pos = 0;
    json[pos++] = '[';
    for (size_t i = 0; i < count; ++i) {
        if (i > 0) json[pos++] = ',';
        json[pos++] = '[';
        for (size_t d = 0; d < EMBED_DIM; ++d) {
            if (d > 0) json[pos++] = ',';
            json[pos++] = '0';
            json[pos++] = '.';
            json[pos++] = '0';
        }
        json[pos++] = ']';
    }
    json[pos++] = ']';
    json[pos] = '\0';

    *out_result = json;
    return 0;
}

static void name_fn(const void *user_data, char **out_name) {
    (void)user_data;
    *out_name = dup_cstr("my-embedder");
}

static void version_fn(const void *user_data, char **out_version) {
    (void)user_data;
    *out_version = dup_cstr("1.0.0");
}

int main(void) {
    KREUZBERGKreuzbergEmbeddingBackendVTable vtable = {0};
    vtable.name_fn = name_fn;
    vtable.version_fn = version_fn;
    vtable.dimensions = dimensions_fn;
    vtable.embed = embed_fn;

    char *err = NULL;
    int32_t rc = kreuzberg_register_embedding_backend(
        "my-embedder",
        vtable,
        NULL,
        &err
    );
    if (rc != 0) {
        fprintf(stderr, "register embedding backend failed: %s\n",
                err ? err : "(no detail)");
        kreuzberg_free_string(err);
        return 1;
    }

    printf("my-embedder registered (dim=%d)\n", EMBED_DIM);
    return 0;
}
```
