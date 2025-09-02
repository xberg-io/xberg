import tempfile
from pathlib import Path
from typing import Any

import pytest

from kreuzberg import ExtractionConfig, extract_bytes_sync
from kreuzberg._config import build_extraction_config_from_dict, load_config_from_file
from kreuzberg._types import HTMLToMarkdownConfig


def test_html_to_markdown_config_defaults() -> None:
    config = HTMLToMarkdownConfig()
    assert config.preprocess_html is True
    assert config.preprocessing_preset == "aggressive"
    assert config.remove_navigation is True
    assert config.remove_forms is True
    assert config.heading_style == "underlined"
    assert config.escape_asterisks is True


def test_html_to_markdown_config_to_dict() -> None:
    config = HTMLToMarkdownConfig(
        heading_style="atx",
        wrap=True,
        wrap_width=100,
        chunk_callback=None,
    )

    config_dict = config.to_dict()
    assert "chunk_callback" not in config_dict
    assert config_dict["heading_style"] == "atx"
    assert config_dict["wrap"] is True
    assert config_dict["wrap_width"] == 100


def test_html_extraction_with_custom_config() -> None:
    html_content = b"""
    <html>
    <body>
        <h1>Test Title</h1>
        <p>This is a <strong>test</strong> paragraph.</p>
    </body>
    </html>
    """

    custom_config = HTMLToMarkdownConfig(
        heading_style="atx",
        strong_em_symbol="_",
        escape_underscores=False,
    )

    config = ExtractionConfig(html_to_markdown_config=custom_config)
    result = extract_bytes_sync(html_content, mime_type="text/html", config=config)

    assert "# Test Title" in result.content
    assert "__test__" in result.content


def test_html_extraction_with_default_config() -> None:
    html_content = b"""
    <html>
    <body>
        <h1>Test Title</h1>
        <p>This is a paragraph.</p>
    </body>
    </html>
    """

    result = extract_bytes_sync(html_content, mime_type="text/html")

    assert "Test Title" in result.content
    assert "This is a paragraph." in result.content


def test_html_to_markdown_config_from_toml() -> None:
    with tempfile.TemporaryDirectory() as tmpdir:
        config_file = Path(tmpdir) / "kreuzberg.toml"
        config_file.write_text("""
[html_to_markdown]
heading_style = "atx"
strong_em_symbol = "_"
escape_underscores = false
wrap = true
wrap_width = 120
preprocessing_preset = "minimal"
""")

        config_dict = load_config_from_file(config_file)
        extraction_config = build_extraction_config_from_dict(config_dict)

        assert extraction_config.html_to_markdown_config is not None
        assert extraction_config.html_to_markdown_config.heading_style == "atx"
        assert extraction_config.html_to_markdown_config.strong_em_symbol == "_"
        assert extraction_config.html_to_markdown_config.escape_underscores is False
        assert extraction_config.html_to_markdown_config.wrap is True
        assert extraction_config.html_to_markdown_config.wrap_width == 120
        assert extraction_config.html_to_markdown_config.preprocessing_preset == "minimal"


def test_html_to_markdown_config_partial_toml() -> None:
    with tempfile.TemporaryDirectory() as tmpdir:
        config_file = Path(tmpdir) / "kreuzberg.toml"
        config_file.write_text("""
force_ocr = false
extract_tables = true

[html_to_markdown]
heading_style = "atx_closed"
bullets = "-"
""")

        config_dict = load_config_from_file(config_file)
        extraction_config = build_extraction_config_from_dict(config_dict)

        assert extraction_config.html_to_markdown_config is not None
        assert extraction_config.html_to_markdown_config.heading_style == "atx_closed"
        assert extraction_config.html_to_markdown_config.bullets == "-"
        assert extraction_config.html_to_markdown_config.escape_asterisks is True


@pytest.mark.parametrize(
    "heading_style,expected_marker",
    [
        ("underlined", "="),
        ("atx", "# "),
        ("atx_closed", "# "),
    ],
)
def test_html_extraction_with_heading_styles(heading_style: Any, expected_marker: str) -> None:
    html_content = b"<html><body><h1>Main Title</h1></body></html>"

    config = ExtractionConfig(html_to_markdown_config=HTMLToMarkdownConfig(heading_style=heading_style))
    result = extract_bytes_sync(html_content, mime_type="text/html", config=config)

    assert "Main Title" in result.content
    if heading_style == "underlined":
        assert "=" in result.content
    else:
        assert expected_marker in result.content


@pytest.mark.parametrize(
    "preprocessing_preset,should_remove_nav",
    [
        ("minimal", False),
        ("standard", True),
        ("aggressive", True),
    ],
)
def test_preprocessing_presets(preprocessing_preset: Any, should_remove_nav: bool) -> None:
    html_content = b"""
    <html>
    <body>
        <nav>Navigation Menu</nav>
        <main>
            <h1>Content</h1>
            <p>Main content here.</p>
        </main>
    </body>
    </html>
    """

    config = ExtractionConfig(
        html_to_markdown_config=HTMLToMarkdownConfig(
            preprocessing_preset=preprocessing_preset,
            remove_navigation=True,
        )
    )
    result = extract_bytes_sync(html_content, mime_type="text/html", config=config)

    assert "Content" in result.content
    assert "Main content here." in result.content

    assert "Navigation Menu" not in result.content


def test_html_to_markdown_config_with_invalid_literal() -> None:
    config = HTMLToMarkdownConfig(heading_style="invalid_style")  # type: ignore[arg-type]
    assert config.heading_style == "invalid_style"  # type: ignore[comparison-overlap]


def test_config_merging() -> None:
    with tempfile.TemporaryDirectory() as tmpdir:
        config_file = Path(tmpdir) / "kreuzberg.toml"
        config_file.write_text("""
[html_to_markdown]
heading_style = "underlined"
wrap = false
""")

        config_dict = load_config_from_file(config_file)
        build_extraction_config_from_dict(config_dict)

        override_html_config = HTMLToMarkdownConfig(
            heading_style="atx",
            wrap=True,
            wrap_width=100,
        )

        html_content = b"<html><body><h1>Test</h1></body></html>"

        final_config = ExtractionConfig(html_to_markdown_config=override_html_config)

        result = extract_bytes_sync(html_content, mime_type="text/html", config=final_config)

        assert "# Test" in result.content


def test_html_to_markdown_config_immutability() -> None:
    config = HTMLToMarkdownConfig()

    with pytest.raises(AttributeError):
        config.heading_style = "atx"  # type: ignore[misc]

    with pytest.raises(AttributeError):
        config.wrap = True  # type: ignore[misc]
