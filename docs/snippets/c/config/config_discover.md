```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* The C FFI does not expose config-file auto-discovery directly. Load the
 * file contents in your application and pass the JSON to
 * kreuzberg_extraction_config_from_json. For TOML/YAML, convert in your
 * application before calling the FFI. */
static char *read_text_file(const char *path) {
    FILE *fp = fopen(path, "rb");
    if (!fp) {
        return NULL;
    }
    fseek(fp, 0, SEEK_END);
    long size = ftell(fp);
    fseek(fp, 0, SEEK_SET);
    char *buf = (char *)malloc((size_t)size + 1);
    if (!buf) {
        fclose(fp);
        return NULL;
    }
    fread(buf, 1, (size_t)size, fp);
    buf[size] = '\0';
    fclose(fp);
    return buf;
}

int main(void) {
    char *json = read_text_file("kreuzberg.json");
    KREUZBERGExtractionConfig *config = json
        ? kreuzberg_extraction_config_from_json(json)
        : kreuzberg_extraction_config_default();
    free(json);

    if (!config) {
        fprintf(stderr, "config load failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    KREUZBERGExtractionResult *result =
        kreuzberg_extract_file_sync("document.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        kreuzberg_extraction_config_free(config);
        return 1;
    }

    char *content = kreuzberg_extraction_result_content(result);
    printf("%s\n", content ? content : "(empty)");
    kreuzberg_free_string(content);

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
