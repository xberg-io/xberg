from typing import Any, cast

from kreuzberg._pandoc import BLOCK_PARA, META_BLOCKS, META_INLINES, META_LIST, META_STRING, _extract_metadata

AST_FIXTURE_META = {
    "pandoc-api-version": ["major", "minor", "patch"],
    "meta": {
        "msg": {
            "c": [
                {"c": "the", "t": "Str"},
                {"c": [], "t": "Space"},
                {
                    "c": [
                        {"c": "quick", "t": "Str"},
                        {"c": [], "t": "Space"},
                        {"c": [{"c": "brown", "t": "Str"}], "t": "Strong"},
                        {"c": [], "t": "Space"},
                        {"c": "fox", "t": "Str"},
                    ],
                    "t": "Emph",
                },
                {"c": [], "t": "Space"},
                {"c": "jumped", "t": "Str"},
            ],
            "t": "MetaInlines",
        },
        "foo": {"c": "bar", "t": "MetaString"},
    },
    "blocks": [
        {"c": [{"c": "%{foo}", "t": "Str"}], "t": "Para"},
        {"c": [{"c": "Hello", "t": "Str"}, {"c": [], "t": "Space"}, {"c": "%{foo}", "t": "Str"}], "t": "Para"},
        {"c": [{"c": [{"c": "%{msg}", "t": "Str"}], "t": "Para"}], "t": "BlockQuote"},
    ],
}

AST_FIXTURE_COMMENTS = [
    {"unMeta": {}},
    [
        {"c": [{"c": "Hello", "t": "Str"}, {"c": [], "t": "Space"}, {"c": "world.", "t": "Str"}], "t": "Para"},
        {"c": ["html", "<!-- BEGIN COMMENT -->\n"], "t": "RawBlock"},
        {
            "c": [
                {"c": "this", "t": "Str"},
                {"c": [], "t": "Space"},
                {"c": "will", "t": "Str"},
                {"c": [], "t": "Space"},
                {"c": "be", "t": "Str"},
                {"c": [], "t": "Space"},
                {"c": "removed.", "t": "Str"},
            ],
            "t": "Para",
        },
        {"c": ["html", "<!-- END COMMENT -->\n"], "t": "RawBlock"},
        {"c": [{"c": "The", "t": "Str"}, {"c": [], "t": "Space"}, {"c": "end.", "t": "Str"}], "t": "Para"},
    ],
]

AST_FIXTURE_DEFLIST = [
    {"unMeta": {}},
    [
        {
            "c": [
                [
                    [{"c": "banana", "t": "Str"}],
                    [
                        [
                            {
                                "c": [
                                    {"c": "a", "t": "Str"},
                                    {"c": [], "t": "Space"},
                                    {"c": "yellow", "t": "Str"},
                                    {"c": [], "t": "Space"},
                                    {"c": "fruit", "t": "Str"},
                                ],
                                "t": "Plain",
                            }
                        ]
                    ],
                ],
                [
                    [{"c": "carrot", "t": "Str"}],
                    [
                        [
                            {
                                "c": [
                                    {"c": "an", "t": "Str"},
                                    {"c": [], "t": "Space"},
                                    {"c": "orange", "t": "Str"},
                                    {"c": [], "t": "Space"},
                                    {"c": "veggie", "t": "Str"},
                                ],
                                "t": "Plain",
                            }
                        ]
                    ],
                ],
            ],
            "t": "DefinitionList",
        }
    ],
]

AST_FIXTURE_SPECIAL_HEADERS = [
    {"unMeta": {}},
    [
        {
            "c": [
                1,
                ["a-special-header", [], []],
                [
                    {"c": "A", "t": "Str"},
                    {"c": [], "t": "Space"},
                    {"c": [{"c": "special", "t": "Str"}], "t": "Emph"},
                    {"c": [], "t": "Space"},
                    {"c": "header", "t": "Str"},
                ],
            ],
            "t": "Header",
        },
        {
            "c": [
                {"c": "The", "t": "Str"},
                {"c": [], "t": "Space"},
                {"c": "quick", "t": "Str"},
                {"c": [], "t": "Space"},
                {"c": [{"c": "brown", "t": "Str"}], "t": "Emph"},
                {"c": [], "t": "Space"},
                {"c": "fox", "t": "Str"},
                {"c": [], "t": "Space"},
                {"c": "jumpped.", "t": "Str"},
            ],
            "t": "Para",
        },
        {
            "c": [
                {"c": "The", "t": "Str"},
                {"c": [], "t": "Space"},
                {
                    "c": [
                        {"c": "quick", "t": "Str"},
                        {"c": [], "t": "Space"},
                        {"c": [{"c": "brown", "t": "Str"}], "t": "Emph"},
                        {"c": [], "t": "Space"},
                        {"c": "fox", "t": "Str"},
                    ],
                    "t": "Strong",
                },
                {"c": [], "t": "Space"},
                {"c": "jumped.", "t": "Str"},
            ],
            "t": "Para",
        },
        {
            "c": [
                [
                    {
                        "c": [
                            {"c": "Buy", "t": "Str"},
                            {"c": [], "t": "Space"},
                            {"c": [{"c": "milk", "t": "Str"}], "t": "Emph"},
                        ],
                        "t": "Plain",
                    }
                ],
                [
                    {
                        "c": [
                            {"c": "Eat", "t": "Str"},
                            {"c": [], "t": "Space"},
                            {"c": [{"c": "cookies", "t": "Str"}], "t": "Emph"},
                        ],
                        "t": "Plain",
                    }
                ],
            ],
            "t": "BulletList",
        },
    ],
]


def test_extract_metadata_empty() -> None:
    assert _extract_metadata({}) == {}


def test_extract_metadata_basic_string() -> None:
    input_meta = {"title": {"t": META_STRING, "c": "Test Document"}, "version": {"t": META_STRING, "c": "1.0.0"}}
    expected = {"title": "Test Document", "version": "1.0.0"}
    assert _extract_metadata(input_meta) == expected


def test_extract_metadata_empty_string() -> None:
    input_meta = {"title": {"t": META_STRING, "c": ""}, "version": {"t": META_STRING, "c": None}}
    assert _extract_metadata(input_meta) == {}


def test_extract_metadata_inlines() -> None:
    input_meta = {
        "title": {"t": META_INLINES, "c": [{"t": "Str", "c": "Test"}, {"t": "Space"}, {"t": "Str", "c": "Document"}]}
    }
    expected = {"title": "Test Document"}
    assert _extract_metadata(input_meta) == expected


def test_extract_metadata_list() -> None:
    input_meta = {
        "authors": {"t": META_LIST, "c": [{"t": META_STRING, "c": "John Doe"}, {"t": META_STRING, "c": "Jane Smith"}]},
        "keywords": {
            "t": META_LIST,
            "c": [
                {"t": META_INLINES, "c": [{"t": "Str", "c": "test"}]},
                {"t": META_INLINES, "c": [{"t": "Str", "c": "metadata"}]},
            ],
        },
    }
    expected = {"authors": ["John Doe", "Jane Smith"], "keywords": ["test", "metadata"]}
    assert _extract_metadata(input_meta) == expected


def test_extract_metadata_blocks() -> None:
    input_meta = {
        "abstract": {
            "t": META_BLOCKS,
            "c": [
                {"t": BLOCK_PARA, "c": [{"t": "Str", "c": "First"}, {"t": "Space"}, {"t": "Str", "c": "paragraph"}]},
                {"t": BLOCK_PARA, "c": [{"t": "Str", "c": "Second"}, {"t": "Space"}, {"t": "Str", "c": "paragraph"}]},
            ],
        }
    }
    expected = {"abstract": ["First paragraph", "Second paragraph"]}
    assert _extract_metadata(input_meta) == expected


def test_extract_metadata_citations() -> None:
    input_meta = {"blocks": [{"t": "Cite", "c": [[{"citationId": "reference1"}, {"citationId": "reference2"}]]}]}
    expected = {"citations": ["reference1", "reference2"]}
    assert _extract_metadata(input_meta) == expected


def test_extract_metadata_complex() -> None:
    input_meta = {
        "title": {"t": META_STRING, "c": "Test Document"},
        "authors": {
            "t": META_LIST,
            "c": [
                {"t": META_INLINES, "c": [{"t": "Str", "c": "John"}, {"t": "Space"}, {"t": "Str", "c": "Doe"}]},
                {"t": META_STRING, "c": "Jane Smith"},
            ],
        },
        "abstract": {
            "t": META_BLOCKS,
            "c": [{"t": BLOCK_PARA, "c": [{"t": "Str", "c": "Test"}, {"t": "Space"}, {"t": "Str", "c": "abstract"}]}],
        },
        "empty": {"t": META_STRING, "c": ""},
        "invalid_key": {"t": "InvalidType", "c": "Something"},
    }
    expected = {"title": "Test Document", "authors": ["John Doe", "Jane Smith"], "abstract": ["Test abstract"]}
    assert _extract_metadata(input_meta) == expected


def test_extract_metadata_invalid_types() -> None:
    input_meta = {
        "title": None,
        "authors": "Not a dict",
        "keywords": {"t": "UnknownType", "c": "content"},
        "abstract": {"t": META_BLOCKS, "c": "Not a list"},
    }
    assert _extract_metadata(input_meta) == {}


def test_extract_metadata_from_meta_fixture() -> None:
    result = _extract_metadata(cast(dict[str, Any], AST_FIXTURE_META["meta"]))
    expected = {
        "foo": "bar",
        # The msg field contains "the quick *brown* fox jumped"
        "msg": "the quick brown fox jumped",
    }
    assert result == expected


def test_extract_metadata_from_special_headers() -> None:
    result = _extract_metadata(cast(dict[str, Any], AST_FIXTURE_SPECIAL_HEADERS[0]))
    assert result == {}


def test_extract_metadata_from_comments() -> None:
    result = _extract_metadata(cast(dict[str, Any], AST_FIXTURE_COMMENTS[0]))
    assert result == {}


def test_extract_metadata_from_deflist() -> None:
    result = _extract_metadata(cast(dict[str, Any], AST_FIXTURE_DEFLIST[0]))
    assert result == {}
