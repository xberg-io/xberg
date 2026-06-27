```c title="C"
#include "xberg.h"
#include <stdio.h>

int main(void) {
    struct CExtractionResult *result = xberg_extract_sync("spreadsheet.xlsx");
    if (!result || !result->success) {
        fprintf(stderr, "Error: %s\n", xberg_get_error_details().message);
        return 1;
    }

    if (result->tables_json) {
        printf("Tables (JSON): %s\n", result->tables_json);
    } else {
        printf("No tables found\n");
    }

    xberg_free_result(result);
    return 0;
}
```
