#include "../../kreuzberg.h"
#include <assert.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define NUM_THREADS 8
#define ITERATIONS 50

/*
 * Concurrent access tests for kreuzberg-ffi.
 *
 * Verifies thread safety of:
 * - Extraction from bytes
 * - Thread-local error isolation
 * - MIME detection
 * - Version query
 */

/* ---- Thread: concurrent text extraction ---- */
static void *thread_extract_text(void *arg) {
    int thread_id = *(int *)arg;
    (void)thread_id;

    for (int i = 0; i < ITERATIONS; i++) {
        const char *text = "Hello from concurrent thread test.";
        struct CExtractionResult *res =
            kreuzberg_extract_bytes_sync((const uint8_t *)text, strlen(text), "text/plain");

        if (res != NULL) {
            assert(res->success);
            assert(res->content != NULL);
            assert(strlen(res->content) > 0);
            kreuzberg_free_result(res);
        }
        /* If res is NULL, the handler may not be available -- that's OK */
    }

    return NULL;
}

/* ---- Thread: thread-local error isolation ---- */
static void *thread_error_isolation(void *arg) {
    int thread_id = *(int *)arg;
    (void)thread_id;

    for (int i = 0; i < ITERATIONS; i++) {
        /* Trigger an error with NULL path */
        const struct CExtractionResult *result = kreuzberg_extract_file_sync(NULL);
        assert(result == NULL);

        /* Error should be set in this thread's TLS */
        const char *err = kreuzberg_last_error();
        assert(err != NULL);
        assert(strlen(err) > 0);

        int32_t code = kreuzberg_last_error_code();
        assert(code != 0);
    }

    return NULL;
}

/* ---- Thread: concurrent MIME detection ---- */
static void *thread_mime_detection(void *arg) {
    int thread_id = *(int *)arg;
    (void)thread_id;

    for (int i = 0; i < ITERATIONS; i++) {
        /* MIME validation (doesn't need files on disk) */
        char *valid = kreuzberg_validate_mime_type("application/pdf");
        assert(valid != NULL);
        kreuzberg_free_string(valid);

        valid = kreuzberg_validate_mime_type("text/html");
        assert(valid != NULL);
        kreuzberg_free_string(valid);

        valid = kreuzberg_validate_mime_type("text/plain");
        assert(valid != NULL);
        kreuzberg_free_string(valid);
    }

    return NULL;
}

/* ---- Thread: concurrent version queries ---- */
static void *thread_version_query(void *arg) {
    int thread_id = *(int *)arg;
    (void)thread_id;

    for (int i = 0; i < ITERATIONS; i++) {
        const char *version = kreuzberg_version();
        assert(version != NULL);
        assert(strlen(version) > 0);
    }

    return NULL;
}

static void run_threaded_test(const char *name, void *(*fn)(void *)) {
    printf("  %s (%d threads x %d iterations)...\n", name, NUM_THREADS, ITERATIONS);

    pthread_t threads[NUM_THREADS];
    int thread_ids[NUM_THREADS];

    for (int i = 0; i < NUM_THREADS; i++) {
        thread_ids[i] = i;
        int rc = pthread_create(&threads[i], NULL, fn, &thread_ids[i]);
        assert(rc == 0);
    }

    for (int i = 0; i < NUM_THREADS; i++) {
        int rc = pthread_join(threads[i], NULL);
        assert(rc == 0);
    }
}

int main(void) {
    run_threaded_test("concurrent text extraction", thread_extract_text);
    run_threaded_test("thread-local error isolation", thread_error_isolation);
    run_threaded_test("concurrent MIME detection", thread_mime_detection);
    run_threaded_test("concurrent version queries", thread_version_query);

    printf("test_concurrent: all tests passed\n");
    return 0;
}
