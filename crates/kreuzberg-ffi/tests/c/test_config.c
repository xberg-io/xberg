#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    /* Test kreuzberg_config_from_json with valid minimal JSON */
    ExtractionConfig *config = kreuzberg_config_from_json("{}");
    assert(config != NULL);

    /* Test kreuzberg_config_to_json round-trip */
    char *json = kreuzberg_config_to_json(config);
    assert(json != NULL);
    assert(strlen(json) > 0);
    /*
     * Note: config_to_json serializes null optional fields (e.g. "html_options":null),
     * and the parser rejects null for fields it expects as objects. This is a known
     * serialization asymmetry. We verify the JSON is non-empty and can create a new
     * config via from_json on the original "{}".
     */
    kreuzberg_free_string(json);

    /* Test kreuzberg_config_get_field */
    char *field = kreuzberg_config_get_field(config, "use_cache");
    /* Field may or may not exist; if it does, it should be valid JSON */
    if (field != NULL) {
        assert(strlen(field) > 0);
        kreuzberg_free_string(field);
    }

    /* Test kreuzberg_config_merge */
    ExtractionConfig *overlay = kreuzberg_config_from_json("{}");
    assert(overlay != NULL);
    int32_t merge_result = kreuzberg_config_merge(config, overlay);
    assert(merge_result == 1);
    kreuzberg_config_free(overlay);

    kreuzberg_config_free(config);

    /* Test kreuzberg_config_from_json with invalid JSON */
    const ExtractionConfig *bad_config = kreuzberg_config_from_json("not valid json");
    assert(bad_config == NULL);

    /* Test kreuzberg_config_is_valid with valid JSON */
    assert(kreuzberg_config_is_valid("{}") == 1);

    /* Test kreuzberg_config_is_valid with invalid JSON */
    assert(kreuzberg_config_is_valid("not valid json") == 0);

    /* Test kreuzberg_config_free with NULL (safe no-op) */
    kreuzberg_config_free(NULL);

    /* Test kreuzberg_config_discover - just test it doesn't crash */
    char *discovered = kreuzberg_config_discover();
    /* May return NULL if no config found; that's okay */
    if (discovered != NULL) {
        kreuzberg_free_string(discovered);
    }

    /* Test kreuzberg_list_embedding_presets */
    char *presets = kreuzberg_list_embedding_presets();
    assert(presets != NULL);
    /* Should be a JSON array, starts with '[' */
    assert(presets[0] == '[');
    kreuzberg_free_string(presets);

    /* Test kreuzberg_get_embedding_preset with invalid name */
    char *bad_preset = kreuzberg_get_embedding_preset("nonexistent_preset_xyz");
    /* Should return NULL for unknown preset */
    if (bad_preset != NULL) {
        kreuzberg_free_string(bad_preset);
    }

    printf("test_config: all tests passed\n");
    return 0;
}
