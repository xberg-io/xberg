#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    /*
     * The intern table is pre-populated with common MIME type strings
     * at startup. We test relative changes rather than absolute counts.
     */

    /* Reset intern table */
    kreuzberg_string_intern_reset();

    /* Capture baseline stats (table comes pre-populated) */
    struct CStringInternStats baseline = kreuzberg_string_intern_stats();

    /* Intern a unique string not in the pre-populated set */
    const char *s1 = kreuzberg_intern_string("x-test/unique-string-12345");
    assert(s1 != NULL);
    assert(strcmp(s1, "x-test/unique-string-12345") == 0);

    /* Intern the same string again; pointers must be equal (deduplication) */
    const char *s2 = kreuzberg_intern_string("x-test/unique-string-12345");
    assert(s2 != NULL);
    assert(s1 == s2);

    /* Intern a different unique string; pointer must differ */
    const char *s3 = kreuzberg_intern_string("x-test/another-unique-67890");
    assert(s3 != NULL);
    assert(strcmp(s3, "x-test/another-unique-67890") == 0);
    assert(s3 != s1);

    /* Check stats: we made 3 requests, added 2 unique strings, got 1 cache hit */
    struct CStringInternStats stats = kreuzberg_string_intern_stats();
    assert(stats.unique_count == baseline.unique_count + 2);
    assert(stats.total_requests == 3);
    assert(stats.cache_hits >= 1);
    assert(stats.total_memory_bytes > 0);

    /* Free interned strings */
    kreuzberg_free_interned_string(s1);
    kreuzberg_free_interned_string(s2);
    kreuzberg_free_interned_string(s3);

    /* Reset and verify request counters are zeroed */
    kreuzberg_string_intern_reset();
    stats = kreuzberg_string_intern_stats();
    assert(stats.total_requests == 0);
    assert(stats.cache_hits == 0);
    assert(stats.cache_misses == 0);

    /* Free with NULL is safe (no-op) */
    kreuzberg_free_interned_string(NULL);

    printf("test_string_intern: all tests passed\n");
    return 0;
}
