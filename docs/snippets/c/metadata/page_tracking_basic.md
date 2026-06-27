```c title="C"
#include "xberg.h"
#include <stdio.h>
#include <string.h>

int main(void) {
    const char *config_json =
        "{"
        "\"pages\": {"
        "\"extract_pages\": true"
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

    char *pages_json = xberg_extraction_result_pages(result);
    if (pages_json) {
        printf("Pages (JSON array): %s\n", pages_json);
        xberg_free_string(pages_json);
    } else {
        printf("No pages available\n");
    }

    XBERGMetadata *metadata = xberg_extraction_result_metadata(result);
    if (metadata) {
        XBERGPageStructure *pages = xberg_metadata_pages(metadata);
        if (pages) {
            printf("Total page count: %zu\n", xberg_page_structure_total_count(pages));
            xberg_page_structure_free(pages);
        }
        xberg_metadata_free(metadata);
    }

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
