#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    const char *version = kreuzberg_version();
    assert(version != NULL);
    assert(strlen(version) > 0);
    assert(strchr(version, '.') != NULL);

    /* Verify defines match runtime */
    char expected[64];
    snprintf(expected, sizeof(expected), "%d.%d.%d", KREUZBERG_VERSION_MAJOR,
             KREUZBERG_VERSION_MINOR, KREUZBERG_VERSION_PATCH);
    assert(strcmp(version, expected) == 0);
    assert(strcmp(version, KREUZBERG_VERSION) == 0);

    printf("test_version: all tests passed\n");
    return 0;
}
