#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    /* Test error code name functions */
    assert(strcmp(kreuzberg_error_code_name(0), "validation") == 0);
    assert(strcmp(kreuzberg_error_code_name(1), "parsing") == 0);
    assert(strcmp(kreuzberg_error_code_name(2), "ocr") == 0);
    assert(strcmp(kreuzberg_error_code_name(3), "missing_dependency") == 0);
    assert(strcmp(kreuzberg_error_code_name(4), "io") == 0);
    assert(strcmp(kreuzberg_error_code_name(5), "plugin") == 0);
    assert(strcmp(kreuzberg_error_code_name(6), "unsupported_format") == 0);
    assert(strcmp(kreuzberg_error_code_name(7), "internal") == 0);
    assert(strcmp(kreuzberg_error_code_name(99), "unknown") == 0);

    /* Test error code description */
    assert(strcmp(kreuzberg_error_code_description(0), "Input validation error") == 0);
    assert(strcmp(kreuzberg_error_code_description(7), "Internal library error") == 0);
    assert(strcmp(kreuzberg_error_code_description(99), "Unknown error code") == 0);

    /* Test error code count */
    assert(kreuzberg_error_code_count() == 8);

    /* Test individual error code accessors */
    assert(kreuzberg_error_code_validation() == 0);
    assert(kreuzberg_error_code_parsing() == 1);
    assert(kreuzberg_error_code_ocr() == 2);
    assert(kreuzberg_error_code_missing_dependency() == 3);
    assert(kreuzberg_error_code_io() == 4);
    assert(kreuzberg_error_code_plugin() == 5);
    assert(kreuzberg_error_code_unsupported_format() == 6);
    assert(kreuzberg_error_code_internal() == 7);

    printf("test_error: all tests passed\n");
    return 0;
}
