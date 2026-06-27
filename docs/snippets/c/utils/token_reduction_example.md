```c title="C"
#include "xberg.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    const char *config_json =
        "{"
        "\"token_reduction\": {"
        "\"mode\": \"moderate\","
        "\"preserve_important_words\": true"
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
        xberg_extract_sync("verbose_document.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        xberg_extraction_config_free(config);
        return 1;
    }

    char *content = xberg_extraction_result_content(result);
    if (content) {
        printf("reduced content (%zu bytes):\n%s\n", strlen(content), content);
        xberg_free_string(content);
    }

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
