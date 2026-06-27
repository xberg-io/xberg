```c title="C"
#include "xberg.h"
#include <stdio.h>

int main(void) {
    const char *config_json =
        "{"
        "\"chunking\": {"
        "\"chunker_type\": \"character\","
        "\"max_characters\": 1000,"
        "\"overlap\": 200,"
        "\"embedding\": {"
        "\"model\": {\"preset\": {\"name\": \"balanced\"}},"
        "\"batch_size\": 16,"
        "\"normalize\": true,"
        "\"show_download_progress\": true"
        "}"
        "}"
        "}";

    XBERGExtractionConfig *config = xberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        return 1;
    }

    XBERGExtractionResult *result =
        xberg_extract_sync("document.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        xberg_extraction_config_free(config);
        return 1;
    }

    char *content = xberg_extraction_result_content(result);
    printf("%s\n", content ? content : "(empty)");
    xberg_free_string(content);

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
