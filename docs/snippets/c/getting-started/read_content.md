```c title="C"
#include <xberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    XBERGExtractionResult *result =
        xberg_extract("document.pdf", NULL, NULL);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        return 1;
    }

    char *content = xberg_extraction_result_content(result);
    if (content) {
        printf("content length: %zu bytes\n", strlen(content));
        printf("%s\n", content);
    }
    xberg_free_string(content);

    /* Tables are returned as a JSON array string. A real consumer would
     * feed this into a JSON parser and walk each table's grid. */
    char *tables_json = xberg_extraction_result_tables(result);
    if (tables_json) {
        printf("tables JSON (%zu bytes):\n%s\n",
               strlen(tables_json), tables_json);
    } else {
        printf("tables JSON: (none)\n");
    }
    xberg_free_string(tables_json);

    xberg_extraction_result_free(result);
    return 0;
}
```
