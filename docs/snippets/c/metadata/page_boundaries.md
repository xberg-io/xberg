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

    char *content = xberg_extraction_result_content(result);
    if (content) {
        printf("Total content length: %zu bytes\n", strlen(content));
        xberg_free_string(content);
    }

    XBERGMetadata *metadata = xberg_extraction_result_metadata(result);
    if (metadata) {
        XBERGPageStructure *pages = xberg_metadata_pages(metadata);
        if (pages) {
            printf("Total pages: %zu\n", xberg_page_structure_total_count(pages));

            char *boundaries_json = xberg_page_structure_boundaries(pages);
            if (boundaries_json) {
                printf("Page boundaries (JSON): %s\n", boundaries_json);
                xberg_free_string(boundaries_json);
            } else {
                printf("No page boundaries available\n");
            }
            xberg_page_structure_free(pages);
        } else {
            printf("No page structure available\n");
        }
        xberg_metadata_free(metadata);
    }

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
