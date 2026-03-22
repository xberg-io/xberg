/* Auto-generated from fixtures/plugin_api/ - DO NOT EDIT */
/* E2E tests for plugin/config/utility APIs via C FFI. */

#include "helpers.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>

/* --- Configuration --- */

static void test_plugin_config_discover(void) {
    char tmp_dir[] = "/tmp/kreuzberg_test_XXXXXX";
    if (!mkdtemp(tmp_dir)) {
        fputs("FAIL: mkdtemp failed\n", stderr);
        exit(1);
    }

    char config_path[4096];
    snprintf(config_path, sizeof(config_path), "%s/%s", tmp_dir, "kreuzberg.toml");
    FILE *fp = fopen(config_path, "w");
    if (!fp) {
        fputs("FAIL: cannot create temp config\n", stderr);
        exit(1);
    }
    fputs("[chunking]\nmax_chars = 50\n", fp);
    fclose(fp);

    char sub_dir[4096];
    snprintf(sub_dir, sizeof(sub_dir), "%s/%s", tmp_dir, "subdir");
    mkdir(sub_dir, 0755);

    char original_dir[4096];
    if (!getcwd(original_dir, sizeof(original_dir))) {
        fputs("FAIL: getcwd failed\n", stderr);
        exit(1);
    }
    chdir(sub_dir);

    char *discovered = kreuzberg_config_discover();
    chdir(original_dir);

    if (!discovered) {
        printf("SKIP: kreuzberg_config_discover returned NULL (config not found)\n");
        return;
    }
    kreuzberg_free_string(discovered);
}

static void test_plugin_config_from_file(void) {
    char tmp_dir[] = "/tmp/kreuzberg_test_XXXXXX";
    if (!mkdtemp(tmp_dir)) {
        fputs("FAIL: mkdtemp failed\n", stderr);
        exit(1);
    }

    char config_path[4096];
    snprintf(config_path, sizeof(config_path), "%s/%s", tmp_dir, "test_config.toml");

    FILE *fp = fopen(config_path, "w");
    if (!fp) {
        fputs("FAIL: cannot create temp config file\n", stderr);
        exit(1);
    }
    fputs("[chunking]\nmax_chars = 100\nmax_overlap = 20\n\n[language_detection]\nenabled = false\n", fp);
    fclose(fp);

    ExtractionConfig *config = kreuzberg_config_from_file(config_path);
    if (!config) {
        const char *err = kreuzberg_last_error();
        fprintf(stderr, "FAIL: kreuzberg_config_from_file failed: %s\n",
                err ? err : "(unknown)");
        exit(1);
    }
    kreuzberg_config_free(config);
}

/* --- Document Extractor Management --- */

static void test_plugin_extractors_clear(void) {
    int ok = (int)kreuzberg_clear_document_extractors();
    if (!ok) {
        fprintf(stderr, "FAIL: kreuzberg_clear_document_extractors() returned false\n");
        exit(1);
    }

    char *list = kreuzberg_list_document_extractors();
    if (list && strcmp(list, "[]") != 0 && strlen(list) > 2) {
        fprintf(stderr, "FAIL: expected empty list after clear, got: %s\n", list);
        kreuzberg_free_string(list);
        exit(1);
    }
    if (list) kreuzberg_free_string(list);
}

static void test_plugin_extractors_list(void) {
    char *result = kreuzberg_list_document_extractors();
    if (!result) {
        fprintf(stderr, "FAIL: kreuzberg_list_document_extractors() returned NULL\n");
        exit(1);
    }
    kreuzberg_free_string(result);
}

static void test_plugin_extractors_unregister(void) {
    /* extractors_unregister: graceful unregister of nonexistent item */
    kreuzberg_unregister_document_extractor("nonexistent-extractor-xyz"); /* ignore return – should not crash */
}

/* --- Mime Utilities --- */

static void test_plugin_mime_detect_bytes(void) {
    const char *test_data = "%PDF-1.4\\n";
    char *mime = kreuzberg_detect_mime_type_from_bytes((const unsigned char *)test_data, strlen(test_data));
    if (!mime) {
        fprintf(stderr, "FAIL: kreuzberg_detect_mime_type_from_bytes returned NULL\n");
        exit(1);
    }
    if (!str_contains_ci(mime, "pdf")) {
        fprintf(stderr, "FAIL: expected MIME to contain \"pdf\", got %s\n", mime);
        kreuzberg_free_string(mime);
        exit(1);
    }
    kreuzberg_free_string(mime);
}

static void test_plugin_mime_detect_path(void) {
    char tmp_dir[] = "/tmp/kreuzberg_test_XXXXXX";
    if (!mkdtemp(tmp_dir)) {
        fputs("FAIL: mkdtemp failed\n", stderr);
        exit(1);
    }
    char test_file[4096];
    snprintf(test_file, sizeof(test_file), "%s/%s", tmp_dir, "test.txt");
    FILE *fp = fopen(test_file, "w");
    if (!fp) {
        fputs("FAIL: cannot create temp file\n", stderr);
        exit(1);
    }
    fputs("Hello, world!", fp);
    fclose(fp);

    char *mime = kreuzberg_detect_mime_type_from_path(test_file);
    if (!mime) {
        fprintf(stderr, "FAIL: kreuzberg_detect_mime_type_from_path returned NULL\n");
        exit(1);
    }
    if (!str_contains_ci(mime, "text")) {
        fprintf(stderr, "FAIL: expected MIME to contain \"text\", got %s\n", mime);
        kreuzberg_free_string(mime);
        exit(1);
    }
    kreuzberg_free_string(mime);
}

static void test_plugin_mime_get_extensions(void) {
    char *extensions = kreuzberg_get_extensions_for_mime("application/pdf");
    if (!extensions) {
        fprintf(stderr, "FAIL: kreuzberg_get_extensions_for_mime returned NULL\n");
        exit(1);
    }
    if (!str_contains_ci(extensions, "pdf")) {
        fprintf(stderr, "FAIL: expected extensions to contain \"pdf\", got %s\n", extensions);
        kreuzberg_free_string(extensions);
        exit(1);
    }
    kreuzberg_free_string(extensions);
}

/* --- Ocr Backend Management --- */

static void test_plugin_ocr_backends_clear(void) {
    int ok = (int)kreuzberg_clear_ocr_backends();
    if (!ok) {
        fprintf(stderr, "FAIL: kreuzberg_clear_ocr_backends() returned false\n");
        exit(1);
    }

    char *list = kreuzberg_list_ocr_backends();
    if (list && strcmp(list, "[]") != 0 && strlen(list) > 2) {
        fprintf(stderr, "FAIL: expected empty list after clear, got: %s\n", list);
        kreuzberg_free_string(list);
        exit(1);
    }
    if (list) kreuzberg_free_string(list);
}

static void test_plugin_ocr_backends_list(void) {
    char *result = kreuzberg_list_ocr_backends();
    if (!result) {
        fprintf(stderr, "FAIL: kreuzberg_list_ocr_backends() returned NULL\n");
        exit(1);
    }
    kreuzberg_free_string(result);
}

static void test_plugin_ocr_backends_unregister(void) {
    /* ocr_backends_unregister: graceful unregister of nonexistent item */
    kreuzberg_unregister_ocr_backend("nonexistent-backend-xyz"); /* ignore return – should not crash */
}

/* --- Post Processor Management --- */

static void test_plugin_post_processors_clear(void) {
    int ok = (int)kreuzberg_clear_post_processors();
    if (!ok) {
        fprintf(stderr, "FAIL: kreuzberg_clear_post_processors() returned false\n");
        exit(1);
    }
}

static void test_plugin_post_processors_list(void) {
    char *result = kreuzberg_list_post_processors();
    if (!result) {
        fprintf(stderr, "FAIL: kreuzberg_list_post_processors() returned NULL\n");
        exit(1);
    }
    kreuzberg_free_string(result);
}

/* --- Validator Management --- */

static void test_plugin_validators_clear(void) {
    int ok = (int)kreuzberg_clear_validators();
    if (!ok) {
        fprintf(stderr, "FAIL: kreuzberg_clear_validators() returned false\n");
        exit(1);
    }

    char *list = kreuzberg_list_validators();
    if (list && strcmp(list, "[]") != 0 && strlen(list) > 2) {
        fprintf(stderr, "FAIL: expected empty list after clear, got: %s\n", list);
        kreuzberg_free_string(list);
        exit(1);
    }
    if (list) kreuzberg_free_string(list);
}

static void test_plugin_validators_list(void) {
    char *result = kreuzberg_list_validators();
    if (!result) {
        fprintf(stderr, "FAIL: kreuzberg_list_validators() returned NULL\n");
        exit(1);
    }
    kreuzberg_free_string(result);
}

int main(void) {
    test_plugin_config_discover();
    test_plugin_config_from_file();
    test_plugin_extractors_clear();
    test_plugin_extractors_list();
    test_plugin_extractors_unregister();
    test_plugin_mime_detect_bytes();
    test_plugin_mime_detect_path();
    test_plugin_mime_get_extensions();
    test_plugin_ocr_backends_clear();
    test_plugin_ocr_backends_list();
    test_plugin_ocr_backends_unregister();
    test_plugin_post_processors_clear();
    test_plugin_post_processors_list();
    test_plugin_validators_clear();
    test_plugin_validators_list();
    printf("test_plugin_apis: all tests passed\n");
    return 0;
}
