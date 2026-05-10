```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const char *config_json =
        "{"
        "\"model\": {\"preset\": {\"name\": \"balanced\"}},"
        "\"normalize\": true"
        "}";

    KREUZBERGEmbeddingConfig *config = kreuzberg_embedding_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    /* Embed input is a JSON-encoded array of strings. */
    const char *texts_json = "[\"Hello, world!\", \"Kreuzberg is fast\"]";

    char *embeddings_json = kreuzberg_embed_texts(texts_json, config);
    if (!embeddings_json) {
        fprintf(stderr, "embedding failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        kreuzberg_embedding_config_free(config);
        return 1;
    }

    printf("embeddings (JSON, 2D float array):\n%s\n", embeddings_json);
    kreuzberg_free_string(embeddings_json);

    kreuzberg_embedding_config_free(config);
    return 0;
}
```
