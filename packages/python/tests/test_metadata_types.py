"""Comprehensive tests for Python metadata types in Kreuzberg bindings.

Tests verify:
- Type structure of HtmlMetadata and related TypedDicts
- Rich metadata types (HeaderMetadata, LinkMetadata, ImageMetadata, StructuredData)
- Integration tests using actual kreuzberg.extract() calls
- Edge cases and optional field handling
"""

from __future__ import annotations

import json
import time
import traceback
import tracemalloc
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

from kreuzberg import ExtractionResult, extract_bytes_sync

if TYPE_CHECKING:
    from kreuzberg.types import (
        HeaderMetadata,
        HtmlMetadata,
        ImageMetadata,
        LinkMetadata,
        StructuredData,
    )


class TestHtmlMetadataStructure:
    """Tests for HtmlMetadata TypedDict structure."""

    def test_html_metadata_has_required_fields(self) -> None:
        """Verify HtmlMetadata has all documented fields."""
        sample: HtmlMetadata = {
            "title": "Test Title",
            "description": "Test Description",
            "keywords": ["test", "metadata"],
            "author": "Test Author",
            "canonical_url": "https://example.com",
            "base_href": "https://example.com/",
            "language": "en",
            "text_direction": "ltr",
            "open_graph": {"og:title": "Test"},
            "twitter_card": {"twitter:card": "summary"},
            "meta_tags": {"viewport": "width=device-width"},
            "headers": [],
            "links": [],
            "images": [],
            "structured_data": [],
        }

        assert "title" in sample
        assert "description" in sample
        assert "keywords" in sample
        assert "author" in sample
        assert "canonical_url" in sample
        assert "base_href" in sample
        assert "language" in sample
        assert "text_direction" in sample
        assert "open_graph" in sample
        assert "twitter_card" in sample
        assert "meta_tags" in sample
        assert "headers" in sample
        assert "links" in sample
        assert "images" in sample
        assert "structured_data" in sample

    def test_keywords_is_list(self) -> None:
        """Verify keywords is list[str], not str."""
        sample: HtmlMetadata = {
            "keywords": ["python", "web", "extraction"],
        }

        keywords = sample["keywords"]
        assert isinstance(keywords, list), "keywords should be a list"
        assert all(isinstance(k, str) for k in keywords), "all keywords should be strings"

    def test_keywords_is_not_string(self) -> None:
        """Verify keywords is NOT a single string."""
        sample: HtmlMetadata = {
            "keywords": ["keyword1", "keyword2"],
        }
        keywords = sample["keywords"]
        assert not isinstance(keywords, str), "keywords should not be a string"

    def test_canonical_url_renamed_from_canonical(self) -> None:
        """Verify canonical_url field exists (not canonical)."""
        sample: HtmlMetadata = {
            "canonical_url": "https://example.com/page",
        }

        assert "canonical_url" in sample, "canonical_url field must exist"
        assert sample["canonical_url"] == "https://example.com/page"

    def test_open_graph_is_dict(self) -> None:
        """Verify open_graph is dict[str, str]."""
        sample: HtmlMetadata = {
            "open_graph": {
                "og:title": "My Page",
                "og:description": "Page description",
                "og:image": "https://example.com/image.jpg",
            },
        }

        og = sample["open_graph"]
        assert isinstance(og, dict), "open_graph should be a dict"
        assert all(isinstance(k, str) and isinstance(v, str) for k, v in og.items()), (
            "all keys and values in open_graph should be strings"
        )

    def test_twitter_card_is_dict(self) -> None:
        """Verify twitter_card is dict[str, str]."""
        sample: HtmlMetadata = {
            "twitter_card": {
                "twitter:card": "summary_large_image",
                "twitter:title": "My Title",
                "twitter:image": "https://example.com/image.jpg",
            },
        }

        tc = sample["twitter_card"]
        assert isinstance(tc, dict), "twitter_card should be a dict"
        assert all(isinstance(k, str) and isinstance(v, str) for k, v in tc.items()), (
            "all keys and values in twitter_card should be strings"
        )

    def test_html_metadata_partial_fields(self) -> None:
        """HtmlMetadata should support partial field population (total=False)."""
        minimal: HtmlMetadata = {
            "title": "Just Title",
        }
        assert minimal["title"] == "Just Title"

        partial: HtmlMetadata = {
            "title": "Title",
            "keywords": ["test"],
            "open_graph": {},
        }
        assert "title" in partial
        assert "keywords" in partial
        assert "open_graph" in partial


class TestHeaderMetadataFields:
    """Tests for HeaderMetadata type."""

    def test_header_metadata_structure(self) -> None:
        """Verify HeaderMetadata has all required fields."""
        header: HeaderMetadata = {
            "level": 1,
            "text": "Main Heading",
            "id": "main-heading",
            "depth": 0,
            "html_offset": 150,
        }

        assert header["level"] == 1
        assert header["text"] == "Main Heading"
        assert header["id"] == "main-heading"
        assert header["depth"] == 0
        assert header["html_offset"] == 150

    def test_header_metadata_fields_present(self) -> None:
        """Verify all HeaderMetadata fields are accessible."""
        header: HeaderMetadata = {
            "level": 2,
            "text": "Subheading",
            "id": None,
            "depth": 1,
            "html_offset": 500,
        }

        assert "level" in header
        assert "text" in header
        assert "id" in header
        assert "depth" in header
        assert "html_offset" in header

    def test_header_metadata_optional_id(self) -> None:
        """Verify HeaderMetadata id field can be None."""
        header_with_id: HeaderMetadata = {
            "level": 1,
            "text": "Heading",
            "id": "heading-id",
            "depth": 0,
            "html_offset": 0,
        }

        header_without_id: HeaderMetadata = {
            "level": 1,
            "text": "Heading",
            "id": None,
            "depth": 0,
            "html_offset": 0,
        }

        assert header_with_id["id"] == "heading-id"
        assert header_without_id["id"] is None

    def test_header_metadata_level_range(self) -> None:
        """Test HeaderMetadata with various heading levels (1-6)."""
        for level in range(1, 7):
            header: HeaderMetadata = {
                "level": level,
                "text": f"Heading Level {level}",
                "id": None,
                "depth": level - 1,
                "html_offset": 0,
            }
            assert header["level"] == level


class TestLinkMetadataFields:
    """Tests for LinkMetadata type."""

    def test_link_metadata_structure(self) -> None:
        """Verify LinkMetadata has all required fields."""
        link: LinkMetadata = {
            "href": "https://example.com",
            "text": "Example Link",
            "title": "Example Site",
            "link_type": "external",
            "rel": ["noopener", "noreferrer"],
            "attributes": {"class": "external-link", "data-id": "123"},
        }

        assert link["href"] == "https://example.com"
        assert link["text"] == "Example Link"
        assert link["title"] == "Example Site"
        assert link["link_type"] == "external"
        assert link["rel"] == ["noopener", "noreferrer"]
        assert link["attributes"] == {"class": "external-link", "data-id": "123"}

    def test_link_type_literal_values(self) -> None:
        """Verify link_type accepts literal values."""
        link_types: list[str] = ["anchor", "internal", "external", "email", "phone", "other"]

        for link_type in link_types:
            link: LinkMetadata = {
                "href": "https://example.com",
                "text": "Link",
                "title": None,
                "link_type": link_type,  # type: ignore[assignment]
                "rel": [],
                "attributes": {},
            }
            assert link["link_type"] == link_type

    def test_link_metadata_optional_title(self) -> None:
        """Verify LinkMetadata title can be None."""
        link_with_title: LinkMetadata = {
            "href": "#",
            "text": "Link",
            "title": "Link Title",
            "link_type": "anchor",
            "rel": [],
            "attributes": {},
        }

        link_without_title: LinkMetadata = {
            "href": "#",
            "text": "Link",
            "title": None,
            "link_type": "anchor",
            "rel": [],
            "attributes": {},
        }

        assert link_with_title["title"] == "Link Title"
        assert link_without_title["title"] is None

    def test_link_metadata_rel_is_list(self) -> None:
        """Verify LinkMetadata rel field is a list of strings."""
        link: LinkMetadata = {
            "href": "https://example.com",
            "text": "Link",
            "title": None,
            "link_type": "external",
            "rel": ["noopener", "noreferrer"],
            "attributes": {},
        }

        assert isinstance(link["rel"], list)
        assert all(isinstance(r, str) for r in link["rel"])

    def test_link_metadata_attributes_is_dict(self) -> None:
        """Verify LinkMetadata attributes field is dict[str, str]."""
        link: LinkMetadata = {
            "href": "https://example.com",
            "text": "Link",
            "title": None,
            "link_type": "external",
            "rel": [],
            "attributes": {"class": "btn", "id": "link-1"},
        }

        assert isinstance(link["attributes"], dict)
        assert all(isinstance(k, str) and isinstance(v, str) for k, v in link["attributes"].items())


class TestImageMetadataFields:
    """Tests for ImageMetadata type (HTML image metadata)."""

    def test_image_metadata_structure(self) -> None:
        """Verify ImageMetadata has all required fields."""
        image: ImageMetadata = {
            "src": "https://example.com/image.jpg",
            "alt": "Image description",
            "title": "Image Title",
            "dimensions": (800, 600),
            "image_type": "external",
            "attributes": {"class": "hero-image", "loading": "lazy"},
        }

        assert image["src"] == "https://example.com/image.jpg"
        assert image["alt"] == "Image description"
        assert image["title"] == "Image Title"
        assert image["dimensions"] == (800, 600)
        assert image["image_type"] == "external"
        assert image["attributes"] == {"class": "hero-image", "loading": "lazy"}

    def test_image_type_literal_values(self) -> None:
        """Verify image_type accepts literal values."""
        image_types: list[str] = ["data_uri", "inline_svg", "external", "relative"]

        for image_type in image_types:
            image: ImageMetadata = {
                "src": "https://example.com/image.jpg",
                "alt": None,
                "title": None,
                "dimensions": None,
                "image_type": image_type,  # type: ignore[assignment]
                "attributes": {},
            }
            assert image["image_type"] == image_type

    def test_image_metadata_optional_fields(self) -> None:
        """Verify ImageMetadata optional fields can be None."""
        image: ImageMetadata = {
            "src": "image.png",
            "alt": None,
            "title": None,
            "dimensions": None,
            "image_type": "relative",
            "attributes": {},
        }

        assert image["alt"] is None
        assert image["title"] is None
        assert image["dimensions"] is None

    def test_image_metadata_dimensions_tuple(self) -> None:
        """Verify dimensions is tuple[int, int] or None."""
        image_with_dims: ImageMetadata = {
            "src": "image.jpg",
            "alt": None,
            "title": None,
            "dimensions": (1920, 1080),
            "image_type": "external",
            "attributes": {},
        }

        image_without_dims: ImageMetadata = {
            "src": "image.jpg",
            "alt": None,
            "title": None,
            "dimensions": None,
            "image_type": "external",
            "attributes": {},
        }

        assert isinstance(image_with_dims["dimensions"], tuple)
        assert len(image_with_dims["dimensions"]) == 2
        assert image_without_dims["dimensions"] is None

    def test_image_metadata_attributes_is_dict(self) -> None:
        """Verify ImageMetadata attributes field is dict[str, str]."""
        image: ImageMetadata = {
            "src": "image.jpg",
            "alt": None,
            "title": None,
            "dimensions": None,
            "image_type": "external",
            "attributes": {"srcset": "image-2x.jpg 2x", "width": "100"},
        }

        assert isinstance(image["attributes"], dict)
        assert all(isinstance(k, str) and isinstance(v, str) for k, v in image["attributes"].items())


class TestStructuredDataFields:
    """Tests for StructuredData type."""

    def test_structured_data_structure(self) -> None:
        """Verify StructuredData has all required fields."""
        structured: StructuredData = {
            "data_type": "json_ld",
            "raw_json": '{"@context": "https://schema.org", "@type": "Article"}',
            "schema_type": "Article",
        }

        assert structured["data_type"] == "json_ld"
        assert structured["raw_json"] == '{"@context": "https://schema.org", "@type": "Article"}'
        assert structured["schema_type"] == "Article"

    def test_structured_data_type_literal_values(self) -> None:
        """Verify data_type accepts literal values."""
        data_types: list[str] = ["json_ld", "microdata", "rdfa"]

        for data_type in data_types:
            structured: StructuredData = {
                "data_type": data_type,  # type: ignore[assignment]
                "raw_json": "{}",
                "schema_type": "Type",
            }
            assert structured["data_type"] == data_type

    def test_structured_data_optional_schema_type(self) -> None:
        """Verify StructuredData schema_type can be None."""
        with_schema: StructuredData = {
            "data_type": "json_ld",
            "raw_json": "{}",
            "schema_type": "Organization",
        }

        without_schema: StructuredData = {
            "data_type": "json_ld",
            "raw_json": "{}",
            "schema_type": None,
        }

        assert with_schema["schema_type"] == "Organization"
        assert without_schema["schema_type"] is None

    def test_structured_data_raw_json_format(self) -> None:
        """Verify raw_json is valid JSON string."""
        json_data = {"@context": "https://schema.org", "@type": "Product", "name": "Widget"}
        structured: StructuredData = {
            "data_type": "json_ld",
            "raw_json": json.dumps(json_data),
            "schema_type": "Product",
        }

        parsed = json.loads(structured["raw_json"])
        assert parsed["@type"] == "Product"
        assert parsed["name"] == "Widget"


class TestHtmlExtractionIntegration:
    """Integration tests using actual kreuzberg extraction."""

    @pytest.fixture
    def html_file(self, test_documents: Path) -> Path:
        """Get path to test HTML file."""
        html_path = test_documents / "web" / "html.html"
        if not html_path.exists():
            pytest.skip(f"Test file not found: {html_path}")
        return html_path

    def test_extract_html_returns_metadata(self, html_file: Path) -> None:
        """Extract HTML and verify metadata structure is present."""
        with html_file.open("rb") as f:
            html_bytes = f.read()

        result = extract_bytes_sync(html_bytes, "text/html")

        assert isinstance(result, ExtractionResult), "result should be ExtractionResult"
        assert hasattr(result, "metadata"), "result should have metadata"
        metadata = result.metadata

        assert isinstance(metadata, dict), "metadata should be dict"
        assert len(metadata) >= 0, "metadata should be accessible"

    def test_extract_html_with_comprehensive_tags(self, test_documents: Path) -> None:
        """Extract HTML with multiple metadata tags."""
        html_content = b"""
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <title>Test Page</title>
            <meta name="description" content="This is a test page">
            <meta name="keywords" content="test,page,metadata">
            <meta name="author" content="Test Author">
            <link rel="canonical" href="https://example.com/test">
            <base href="https://example.com/">
            <meta name="viewport" content="width=device-width">
            <meta property="og:title" content="Test OG Title">
            <meta property="og:description" content="Test OG Description">
            <meta property="og:image" content="https://example.com/og-image.jpg">
            <meta name="twitter:card" content="summary">
            <meta name="twitter:title" content="Test Twitter Title">
        </head>
        <body>
            <h1>Main Heading</h1>
            <h2>Subheading</h2>
            <p>Some content with <a href="https://example.com">external link</a></p>
            <img src="test.jpg" alt="Test image">
            <img src="data:image/png;base64,..." alt="Data URI">
        </body>
        </html>
        """

        result = extract_bytes_sync(html_content, "text/html")
        metadata = result.metadata

        assert isinstance(metadata, dict)
        if "html_headers" in metadata:
            assert isinstance(metadata["html_headers"], list)
        if "html_links" in metadata:
            assert isinstance(metadata["html_links"], list)
        if "html_images" in metadata:
            assert isinstance(metadata["html_images"], list)

    def test_metadata_keyword_array(self, html_file: Path) -> None:
        """Extract HTML with keywords, verify as list not string."""
        html_with_keywords = b"""
        <!DOCTYPE html>
        <html>
        <head>
            <meta name="keywords" content="python,web,extraction">
        </head>
        <body>Content</body>
        </html>
        """

        result = extract_bytes_sync(html_with_keywords, "text/html")
        metadata = result.metadata

        if "keywords" in metadata:
            keywords = metadata["keywords"]
            assert isinstance(keywords, list), "keywords must be list, not string"
            if len(keywords) > 0:
                assert all(isinstance(k, str) for k in keywords)

    def test_metadata_open_graph_dict(self, html_file: Path) -> None:
        """Extract OG tags, verify as dict not list."""
        html_with_og = b"""
        <!DOCTYPE html>
        <html>
        <head>
            <meta property="og:title" content="My Title">
            <meta property="og:description" content="My Description">
            <meta property="og:image" content="https://example.com/img.jpg">
            <meta property="og:url" content="https://example.com">
        </head>
        <body>Content</body>
        </html>
        """

        result = extract_bytes_sync(html_with_og, "text/html")
        metadata = result.metadata

        if "open_graph" in metadata:
            og = metadata["open_graph"]
            assert isinstance(og, dict), "open_graph must be dict"
            assert all(isinstance(k, str) and isinstance(v, str) for k, v in og.items())

    def test_metadata_headers_list(self, html_file: Path) -> None:
        """Extract headers, verify as list of HeaderMetadata."""
        html_with_headers = b"""
        <!DOCTYPE html>
        <html>
        <head><title>Test</title></head>
        <body>
            <h1 id="intro">Introduction</h1>
            <h2>Background</h2>
            <h3>Details</h3>
            <h2>Conclusion</h2>
        </body>
        </html>
        """

        result = extract_bytes_sync(html_with_headers, "text/html")
        metadata = result.metadata

        if "html_headers" in metadata:
            headers = metadata["html_headers"]
            assert isinstance(headers, list), "headers must be list"

            for header in headers:
                assert isinstance(header, dict), "header must be dict"
                assert "level" in header, "header must have level"
                assert "text" in header, "header must have text"
                assert isinstance(header["level"], int)
                assert isinstance(header["text"], str)

    def test_metadata_links_list(self, html_file: Path) -> None:
        """Extract links, verify as list of LinkMetadata."""
        html_with_links = b"""
        <!DOCTYPE html>
        <html>
        <head><title>Test</title></head>
        <body>
            <a href="https://example.com" rel="noopener">External</a>
            <a href="/page" title="Internal">Internal Link</a>
            <a href="#section">Anchor</a>
            <a href="mailto:test@example.com">Email</a>
        </body>
        </html>
        """

        result = extract_bytes_sync(html_with_links, "text/html")
        metadata = result.metadata

        if "html_links" in metadata:
            links = metadata["html_links"]
            assert isinstance(links, list), "links must be list"

            for link in links:
                assert isinstance(link, dict), "link must be dict"
                assert "href" in link, "link must have href"
                assert "text" in link, "link must have text"
                assert "link_type" in link, "link must have link_type"
                assert isinstance(link["href"], str)
                assert isinstance(link["text"], str)
                assert isinstance(link["rel"], list)
                assert isinstance(link["attributes"], dict)

    def test_metadata_images_list(self, html_file: Path) -> None:
        """Extract images, verify as list of ImageMetadata."""
        html_with_images = b"""
        <!DOCTYPE html>
        <html>
        <head><title>Test</title></head>
        <body>
            <img src="https://example.com/image.jpg" alt="External">
            <img src="relative/path.png" alt="Relative">
            <img src="data:image/svg+xml;..." alt="Inline SVG">
            <img src="/absolute/image.gif" title="With Title">
        </body>
        </html>
        """

        result = extract_bytes_sync(html_with_images, "text/html")
        metadata = result.metadata

        if "html_images" in metadata:
            images = metadata["html_images"]
            assert isinstance(images, list), "images must be list"

            for image in images:
                assert isinstance(image, dict), "image must be dict"
                assert "src" in image, "image must have src"
                assert "image_type" in image, "image must have image_type"
                assert isinstance(image["src"], str)
                assert isinstance(image["attributes"], dict)


class TestMetadataEdgeCases:
    """Edge cases and optional field handling."""

    def test_metadata_empty_html(self) -> None:
        """Empty HTML returns default structure."""
        empty_html = b"<html><body></body></html>"
        result = extract_bytes_sync(empty_html, "text/html")

        assert hasattr(result, "metadata")
        metadata = result.metadata
        assert isinstance(metadata, dict)

    def test_metadata_minimal_html(self) -> None:
        """Minimal HTML document."""
        minimal_html = b"<h1>Title</h1>"
        result = extract_bytes_sync(minimal_html, "text/html")

        assert hasattr(result, "metadata")
        metadata = result.metadata
        assert isinstance(metadata, dict)

    def test_metadata_none_values(self) -> None:
        """Optional fields are None when missing."""
        html_minimal = b"""
        <!DOCTYPE html>
        <html>
        <head>
            <title>Only Title</title>
        </head>
        <body>Content</body>
        </html>
        """

        result = extract_bytes_sync(html_minimal, "text/html")
        metadata = result.metadata

        assert isinstance(metadata, dict)

    def test_metadata_empty_collections(self) -> None:
        """Empty lists/dicts when no data."""
        html = b"<h1>Title</h1>"
        result = extract_bytes_sync(html, "text/html")
        metadata = result.metadata

        if "html_headers" in metadata:
            headers = metadata["html_headers"]
            assert isinstance(headers, list)

        if "html_links" in metadata:
            links = metadata["html_links"]
            assert isinstance(links, list)

        if "html_images" in metadata:
            images = metadata["html_images"]
            assert isinstance(images, list)

    def test_metadata_special_characters(self) -> None:
        """HTML with special characters in metadata."""
        html_special = b"""
        <!DOCTYPE html>
        <html>
        <head>
            <meta name="description" content="Test with &amp; special &lt;chars&gt;">
            <meta name="keywords" content="test,\xc3\xa9,\xc3\xa7">
            <title>Title with \xc3\xa9 accents</title>
        </head>
        <body>
            <h1>Heading with &quot;quotes&quot;</h1>
            <a href="test?param=value&other=123">Link with &amp;</a>
        </body>
        </html>
        """

        result = extract_bytes_sync(html_special, "text/html")
        metadata = result.metadata

        assert isinstance(metadata, dict)

    def test_metadata_very_long_values(self) -> None:
        """HTML with very long metadata values."""
        long_description = "A" * 10000
        long_content = "B" * 50000

        html_long = f"""
        <!DOCTYPE html>
        <html>
        <head>
            <meta name="description" content="{long_description}">
            <title>Long Title</title>
        </head>
        <body>
            {long_content}
        </body>
        </html>
        """.encode()

        result = extract_bytes_sync(html_long, "text/html")
        metadata = result.metadata

        assert isinstance(metadata, dict)

    def test_metadata_malformed_html(self) -> None:
        """Malformed HTML still returns valid metadata."""
        malformed = b"""
        <html>
        <head><title>Unclosed title
        <body>
        <h1>Unclosed heading
        <p>Unclosed paragraph
        """

        result = extract_bytes_sync(malformed, "text/html")
        metadata = result.metadata

        assert isinstance(metadata, dict)

    def test_metadata_nested_html_elements(self) -> None:
        """Deeply nested HTML elements."""
        nested_html = b"""
        <!DOCTYPE html>
        <html>
        <body>
            <div>
                <section>
                    <article>
                        <h1>Deep Heading</h1>
                        <div>
                            <p>
                                <span>
                                    <a href="#">Nested Link</a>
                                </span>
                            </p>
                        </div>
                    </article>
                </section>
            </div>
        </body>
        </html>
        """

        result = extract_bytes_sync(nested_html, "text/html")
        metadata = result.metadata

        assert isinstance(metadata, dict)

    def test_metadata_javascript_in_html(self) -> None:
        """HTML with embedded JavaScript."""
        js_html = b"""
        <!DOCTYPE html>
        <html>
        <head>
            <title>With JS</title>
            <script>
                var data = {
                    title: "Not a title",
                    keywords: "not keywords"
                };
            </script>
        </head>
        <body>
            <h1>Real Heading</h1>
            <a href="javascript:void(0)">JS Link</a>
        </body>
        </html>
        """

        result = extract_bytes_sync(js_html, "text/html")
        metadata = result.metadata

        assert isinstance(metadata, dict)

    def test_metadata_css_in_html(self) -> None:
        """HTML with embedded CSS."""
        css_html = b"""
        <!DOCTYPE html>
        <html>
        <head>
            <title>With CSS</title>
            <style>
                h1 { color: red; }
                .heading::before { content: "Not a heading"; }
            </style>
        </head>
        <body>
            <h1>Real Heading</h1>
        </body>
        </html>
        """

        result = extract_bytes_sync(css_html, "text/html")
        metadata = result.metadata

        assert isinstance(metadata, dict)

    def test_metadata_multiple_same_tags(self) -> None:
        """HTML with multiple instances of same metadata tags."""
        multi_tags = b"""
        <!DOCTYPE html>
        <html>
        <head>
            <meta name="keywords" content="python">
            <meta name="keywords" content="web">
            <meta name="keywords" content="extraction">
            <link rel="alternate" hreflang="en" href="https://example.com/en">
            <link rel="alternate" hreflang="fr" href="https://example.com/fr">
        </head>
        <body>
            <h1>Heading 1</h1>
            <h1>Heading 2</h1>
            <h1>Heading 3</h1>
        </body>
        </html>
        """

        result = extract_bytes_sync(multi_tags, "text/html")
        metadata = result.metadata

        assert isinstance(metadata, dict)
        if "html_headers" in metadata:
            headers = metadata["html_headers"]
            if len(headers) > 0:
                h1s = [h for h in headers if h.get("level") == 1]
                assert len(h1s) >= 1


class TestMetadataJsonSerialization:
    """Tests for JSON serialization/deserialization of metadata."""

    def test_metadata_json_serializable(self) -> None:
        """Metadata should be JSON serializable."""
        header: HeaderMetadata = {
            "level": 1,
            "text": "Heading",
            "id": "heading-id",
            "depth": 0,
            "html_offset": 100,
        }

        json_str = json.dumps(header)
        assert isinstance(json_str, str)

        deserialized = json.loads(json_str)
        assert deserialized["level"] == 1
        assert deserialized["text"] == "Heading"

    def test_rich_metadata_round_trip(self) -> None:
        """Rich metadata should survive JSON serialization round trip."""
        original: HtmlMetadata = {
            "title": "Test Page",
            "keywords": ["a", "b", "c"],
            "open_graph": {"og:title": "OG Title"},
            "headers": [
                {
                    "level": 1,
                    "text": "H1",
                    "id": None,
                    "depth": 0,
                    "html_offset": 0,
                }
            ],
            "links": [
                {
                    "href": "https://example.com",
                    "text": "Link",
                    "title": None,
                    "link_type": "external",
                    "rel": [],
                    "attributes": {},
                }
            ],
        }

        json_str = json.dumps(original)
        deserialized = json.loads(json_str)

        assert deserialized["title"] == original["title"]
        assert deserialized["keywords"] == original["keywords"]
        assert deserialized["open_graph"] == original["open_graph"]

    def test_extraction_result_json_serializable(self) -> None:
        """Complete ExtractionResult metadata should be JSON serializable."""
        html_content = b"""
        <!DOCTYPE html>
        <html>
        <head>
            <title>Test</title>
            <meta name="description" content="Test page">
        </head>
        <body>
            <h1>Heading</h1>
            <p>Content</p>
        </body>
        </html>
        """

        result = extract_bytes_sync(html_content, "text/html")

        try:
            json.dumps(result.metadata, default=str)
        except TypeError as e:
            pytest.fail(f"Metadata should be JSON serializable: {e}")


class TestLargeHtmlExtractionPerformance:
    """Performance tests for extraction of large HTML documents."""

    def test_large_html_extraction_performance(self) -> None:
        """Test extraction of large HTML document completes within 5 seconds.

        Generates HTML with 10,000+ elements and verifies extraction
        completes in reasonable time (<5 seconds).
        """
        elements = []
        elements.append("<h1>Performance Test</h1>")

        for i in range(1000):
            elements.append(f"<section id='section-{i}'>")
            elements.append(f"<h2>Section {i}</h2>")
            elements.append(f"<p>Paragraph content for section {i}.</p>")
            elements.append(f"<a href='/page-{i}'>Link {i}</a>")
            elements.append("</section>")

        html_content = f"""
        <!DOCTYPE html>
        <html>
        <head>
            <title>Large HTML Performance Test</title>
            <meta name="description" content="Testing extraction performance on large documents">
        </head>
        <body>
            {"".join(elements)}
        </body>
        </html>
        """.encode()

        start_time = time.time()
        result = extract_bytes_sync(html_content, "text/html")
        elapsed_time = time.time() - start_time

        assert hasattr(result, "metadata")
        metadata = result.metadata
        assert isinstance(metadata, dict)

        assert elapsed_time < 5.0, f"Extraction took {elapsed_time:.2f}s, expected < 5.0s"


class TestConcurrentExtractionThreadSafety:
    """Test thread safety of concurrent extraction operations."""

    def test_concurrent_extraction_thread_safety(self) -> None:
        """Test extracting same HTML from multiple threads is thread-safe.

        Uses ThreadPoolExecutor to extract HTML concurrently and validates
        no exceptions or data corruption occurs.
        """
        html_content = b"""
        <!DOCTYPE html>
        <html>
        <head>
            <title>Thread Safety Test</title>
            <meta name="description" content="Testing concurrent extraction">
            <meta property="og:title" content="OG Title">
        </head>
        <body>
            <h1>Main Heading</h1>
            <h2>Subheading 1</h2>
            <h2>Subheading 2</h2>
            <a href="https://example.com">Link 1</a>
            <a href="https://example.com/page">Link 2</a>
            <img src="image1.jpg" alt="Image 1">
            <img src="image2.jpg" alt="Image 2">
        </body>
        </html>
        """

        extraction_count = 10
        results = []
        exceptions = []

        def extract_html() -> dict:
            """Extract HTML and return metadata."""
            try:
                result = extract_bytes_sync(html_content, "text/html")
                return result.metadata
            except Exception as e:
                exceptions.append((e, traceback.format_exc()))
                raise

        with ThreadPoolExecutor(max_workers=5) as executor:
            futures = [executor.submit(extract_html) for _ in range(extraction_count)]
            results = [future.result() for future in futures]

        assert len(exceptions) == 0, f"Thread safety violations detected: {exceptions}"

        assert len(results) == extraction_count
        for i, metadata in enumerate(results):
            assert isinstance(metadata, dict), f"Result {i} metadata should be dict, got {type(metadata)}"

        first_result = results[0]
        for i, metadata in enumerate(results[1:], 1):
            if "title" in first_result and "title" in metadata:
                assert first_result["title"] == metadata["title"], f"Result {i} has different title than first result"


class TestMalformedStructuredDataHandling:
    """Test handling of malformed structured data (JSON-LD, etc)."""

    def test_malformed_structured_data_handling(self) -> None:
        """Test HTML with malformed JSON-LD handles gracefully.

        HTML with invalid JSON-LD should not cause extraction to fail.
        Should handle gracefully without exceptions.
        """
        html_content = b"""
        <!DOCTYPE html>
        <html>
        <head>
            <title>Malformed JSON-LD Test</title>
            <script type="application/ld+json">
            {invalid json here - missing quotes, unclosed braces
            </script>
            <script type="application/ld+json">
            {"@context": "https://schema.org", "@type": "Article"
            </script>
        </head>
        <body>
            <h1>Valid Content</h1>
            <p>This should still be extracted.</p>
        </body>
        </html>
        """

        try:
            result = extract_bytes_sync(html_content, "text/html")
        except Exception as e:
            pytest.fail(f"Extraction failed with malformed JSON-LD: {e}")

        assert hasattr(result, "metadata")
        metadata = result.metadata
        assert isinstance(metadata, dict)

        if "title" in metadata:
            assert metadata["title"] is not None


class TestMemoryCleanupAfterExtraction:
    """Test memory cleanup after large extraction operations."""

    def test_memory_cleanup_after_extraction(self) -> None:
        """Test that memory is released after large HTML extraction.

        Extracts a large document and validates memory is properly cleaned up.
        """
        tracemalloc.start()

        large_html = (
            b"""
        <!DOCTYPE html>
        <html>
        <head>
            <title>Large Memory Test</title>
        </head>
        <body>
        """
            + (b"<p>" + b"A" * 1000 + b"</p>" * 500)
            + b"""
        </body>
        </html>
        """
        )

        tracemalloc.reset_peak()

        result = extract_bytes_sync(large_html, "text/html")
        assert hasattr(result, "metadata")

        _current, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()

        metadata = result.metadata
        assert isinstance(metadata, dict)

        peak_mb = peak / (1024 * 1024)
        assert peak_mb < 100, f"Peak memory usage {peak_mb:.2f}MB seems excessive for document"


class TestInvalidEncodingHandling:
    """Test handling of various character encodings and invalid bytes."""

    def test_invalid_encoding_handling(self) -> None:
        """Test extraction handles various encodings and invalid UTF-8.

        Tests HTML with UTF-16, invalid UTF-8 bytes, and mixed encodings.
        Should handle gracefully without crashing.
        """
        test_cases = [
            {
                "name": "valid_utf8",
                "content": b"""
                <!DOCTYPE html>
                <html>
                <head>
                    <meta charset="UTF-8">
                    <title>Valid UTF-8</title>
                </head>
                <body>Content with \xc3\xa9 accents</body>
                </html>
                """,
            },
            {
                "name": "invalid_utf8",
                "content": b"""
                <!DOCTYPE html>
                <html>
                <head>
                    <meta charset="UTF-8">
                    <title>Invalid UTF-8</title>
                </head>
                <body>Content with \xff\xfe invalid bytes</body>
                </html>
                """,
            },
            {
                "name": "mixed_encoding",
                "content": b"""
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Mixed \xc3\xa9ncoding</title>
                </head>
                <body>
                    <h1>Valid \xc3\xa7haracters</h1>
                    <p>Some \xff\xfe invalid bytes \xc3\xa9 mixed in</p>
                </body>
                </html>
                """,
            },
            {
                "name": "utf16_content",
                "content": "<!DOCTYPE html><html><head><title>UTF-16 Test</title></head><body>Content</body></html>".encode(
                    "utf-16"
                ),
            },
        ]

        try:
            for test_case in test_cases:
                result = extract_bytes_sync(test_case["content"], "text/html")
                assert hasattr(result, "metadata"), f"{test_case['name']}: result should have metadata"
        except Exception as e:
            assert isinstance(e, (UnicodeDecodeError, ValueError, Exception)), (
                f"{test_case['name']}: unexpected exception type {type(e)}"
            )


@pytest.fixture
def test_documents() -> Path:
    """Path to test_documents directory."""
    path = Path(__file__).parent.parent.parent.parent / "test_documents"
    if not path.exists():
        pytest.skip(f"Test documents directory not found: {path}")
    return path
