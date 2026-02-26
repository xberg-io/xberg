#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    int32_t val;
    const char *str;

    /* heading_style: valid values "atx", "underlined", "atx_closed" / "atx-closed" */
    val = kreuzberg_parse_heading_style("atx");
    assert(val >= 0);
    str = kreuzberg_heading_style_to_string(val);
    assert(str != NULL);

    val = kreuzberg_parse_heading_style("underlined");
    assert(val >= 0);
    str = kreuzberg_heading_style_to_string(val);
    assert(str != NULL);

    /* Invalid heading style */
    val = kreuzberg_parse_heading_style("invalid_value");
    assert(val < 0);

    /* NULL returns invalid */
    val = kreuzberg_parse_heading_style(NULL);
    assert(val < 0);

    /* Invalid discriminant returns NULL */
    str = kreuzberg_heading_style_to_string(-1);
    assert(str == NULL);
    str = kreuzberg_heading_style_to_string(999);
    assert(str == NULL);

    /* code_block_style: valid values "indented", "backticks", "tildes" */
    val = kreuzberg_parse_code_block_style("backticks");
    assert(val >= 0);
    str = kreuzberg_code_block_style_to_string(val);
    assert(str != NULL);

    val = kreuzberg_parse_code_block_style("indented");
    assert(val >= 0);

    val = kreuzberg_parse_code_block_style("tildes");
    assert(val >= 0);

    val = kreuzberg_parse_code_block_style("invalid_value");
    assert(val < 0);

    /* highlight_style: valid values "double_equal"/"=="/"double-equal", "html", "bold", "none" */
    val = kreuzberg_parse_highlight_style("html");
    assert(val >= 0);
    str = kreuzberg_highlight_style_to_string(val);
    assert(str != NULL);

    val = kreuzberg_parse_highlight_style("bold");
    assert(val >= 0);

    val = kreuzberg_parse_highlight_style("none");
    assert(val >= 0);

    val = kreuzberg_parse_highlight_style("invalid_value");
    assert(val < 0);

    /* list_indent_type: valid values "spaces", "tabs" */
    val = kreuzberg_parse_list_indent_type("spaces");
    assert(val >= 0);
    str = kreuzberg_list_indent_type_to_string(val);
    assert(str != NULL);

    val = kreuzberg_parse_list_indent_type("tabs");
    assert(val >= 0);

    val = kreuzberg_parse_list_indent_type("invalid_value");
    assert(val < 0);

    /* whitespace_mode: valid values "default", "preserve", "preserve_inner", "collapse" */
    val = kreuzberg_parse_whitespace_mode("default");
    assert(val >= 0);
    str = kreuzberg_whitespace_mode_to_string(val);
    assert(str != NULL);

    val = kreuzberg_parse_whitespace_mode("preserve");
    assert(val >= 0);

    val = kreuzberg_parse_whitespace_mode("collapse");
    assert(val >= 0);

    val = kreuzberg_parse_whitespace_mode("invalid_value");
    assert(val < 0);

    /* newline_style: valid values "default", "spaces", "backslash" */
    val = kreuzberg_parse_newline_style("default");
    assert(val >= 0);
    str = kreuzberg_newline_style_to_string(val);
    assert(str != NULL);

    val = kreuzberg_parse_newline_style("spaces");
    assert(val >= 0);

    val = kreuzberg_parse_newline_style("backslash");
    assert(val >= 0);

    val = kreuzberg_parse_newline_style("invalid_value");
    assert(val < 0);

    /* preprocessing_preset: valid values "none", "conservative", "aggressive" */
    val = kreuzberg_parse_preprocessing_preset("none");
    assert(val >= 0);
    str = kreuzberg_preprocessing_preset_to_string(val);
    assert(str != NULL);

    val = kreuzberg_parse_preprocessing_preset("conservative");
    assert(val >= 0);

    val = kreuzberg_parse_preprocessing_preset("aggressive");
    assert(val >= 0);

    val = kreuzberg_parse_preprocessing_preset("invalid_value");
    assert(val < 0);

    printf("test_html_options: all tests passed\n");
    return 0;
}
