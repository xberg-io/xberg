```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <string.h>

int main(void) {
    const char *config_json =
        "{"
        "\"pages\": {"
        "\"extract_pages\": true"
        "}"
        "}";

    KREUZBERGExtractionConfig *config = kreuzberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
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
    if (content) {
        printf("Total content length: %zu bytes\n", strlen(content));
        kreuzberg_free_string(content);
    }

    KREUZBERGMetadata *metadata = kreuzberg_extraction_result_metadata(result);
    if (metadata) {
        KREUZBERGPageStructure *pages = kreuzberg_metadata_pages(metadata);
        if (pages) {
            printf("Total pages: %zu\n", kreuzberg_page_structure_total_count(pages));

            char *boundaries_json = kreuzberg_page_structure_boundaries(pages);
            if (boundaries_json) {
                printf("Page boundaries (JSON): %s\n", boundaries_json);
                kreuzberg_free_string(boundaries_json);
            } else {
                printf("No page boundaries available\n");
            }
            kreuzberg_page_structure_free(pages);
        } else {
            printf("No page structure available\n");
        }
        kreuzberg_metadata_free(metadata);
    }

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
