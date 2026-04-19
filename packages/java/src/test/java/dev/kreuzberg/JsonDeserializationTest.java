package dev.kreuzberg;

import static org.junit.jupiter.api.Assertions.*;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;
import java.util.List;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;

/**
 * Regression tests for JSON deserialization with nested generic collections.
 *
 * <p>
 * These tests verify that Jackson correctly deserializes nested List&lt;T&gt;
 * fields to proper Java types instead of LinkedHashMap (which would cause
 * ClassCastException at runtime due to Java type erasure).
 *
 * <p>
 * Related to GitHub issue #355.
 *
 * @see <a href="https://github.com/kreuzberg-dev/kreuzberg/issues/355">Issue
 *      #355</a>
 */
@DisplayName("JSON Deserialization (Issue #355 Regression)")
class JsonDeserializationTest {

	private ObjectMapper mapper;

	@BeforeEach
	void setUp() {
		mapper = new ObjectMapper().setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE)
				.configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);
	}

	@Nested
	@DisplayName("FormattedBlock deserialization")
	class FormattedBlockTests {

		@Test
		@DisplayName("should deserialize List<InlineElement> correctly")
		void testInlineContentDeserialization() throws Exception {
			String json = """
					{
						"block_type": "paragraph",
						"inline_content": [
							{"element_type": "text", "content": "Hello ", "attributes": null, "metadata": null},
							{"element_type": "emphasis", "content": "world", "attributes": null, "metadata": null}
						],
						"children": []
					}
					""";

			FormattedBlock block = mapper.readValue(json, FormattedBlock.class);

			assertNotNull(block);
			assertEquals(BlockType.PARAGRAPH, block.getBlockType());

			List<InlineElement> inlineContent = block.getInlineContent();
			assertEquals(2, inlineContent.size());

			// This would throw ClassCastException if elements were LinkedHashMap
			InlineElement first = inlineContent.get(0);
			assertEquals("Hello ", first.getContent());
			assertEquals(InlineType.TEXT, first.getElementType());

			InlineElement second = inlineContent.get(1);
			assertEquals("world", second.getContent());
			assertEquals(InlineType.EMPHASIS, second.getElementType());
		}

		@Test
		@DisplayName("should deserialize nested List<FormattedBlock> children correctly")
		void testChildrenDeserialization() throws Exception {
			String json = """
					{
						"block_type": "blockquote",
						"inline_content": [],
						"children": [
							{
								"block_type": "paragraph",
								"inline_content": [
									{"element_type": "text", "content": "Quoted text", "attributes": null, "metadata": null}
								],
								"children": []
							},
							{
								"block_type": "paragraph",
								"inline_content": [
									{"element_type": "text", "content": "More quoted text", "attributes": null, "metadata": null}
								],
								"children": []
							}
						]
					}
					""";

			FormattedBlock block = mapper.readValue(json, FormattedBlock.class);

			assertNotNull(block);
			assertEquals(BlockType.BLOCKQUOTE, block.getBlockType());
			assertTrue(block.hasChildren());

			List<FormattedBlock> children = block.getChildren();
			assertEquals(2, children.size());

			// This would throw ClassCastException if elements were LinkedHashMap
			FormattedBlock firstChild = children.get(0);
			assertEquals(BlockType.PARAGRAPH, firstChild.getBlockType());
			assertEquals("Quoted text", firstChild.getInlineContent().get(0).getContent());

			FormattedBlock secondChild = children.get(1);
			assertEquals(BlockType.PARAGRAPH, secondChild.getBlockType());
			assertEquals("More quoted text", secondChild.getInlineContent().get(0).getContent());
		}

		@Test
		@DisplayName("should deserialize deeply nested blocks")
		void testDeeplyNestedBlocks() throws Exception {
			String json = """
					{
						"block_type": "div",
						"inline_content": [],
						"children": [
							{
								"block_type": "blockquote",
								"inline_content": [],
								"children": [
									{
										"block_type": "paragraph",
										"inline_content": [
											{"element_type": "strong", "content": "Deep", "attributes": null, "metadata": null}
										],
										"children": []
									}
								]
							}
						]
					}
					""";

			FormattedBlock block = mapper.readValue(json, FormattedBlock.class);

			FormattedBlock level1 = block.getChildren().get(0);
			FormattedBlock level2 = level1.getChildren().get(0);
			InlineElement inline = level2.getInlineContent().get(0);

			assertEquals("Deep", inline.getContent());
			assertEquals(InlineType.STRONG, inline.getElementType());
		}
	}

	@Nested
	@DisplayName("Footnote deserialization")
	class FootnoteTests {

		@Test
		@DisplayName("should deserialize List<FormattedBlock> content correctly")
		void testFootnoteContentDeserialization() throws Exception {
			String json = """
					{
						"label": "fn1",
						"content": [
							{
								"block_type": "paragraph",
								"inline_content": [
									{"element_type": "text", "content": "Footnote text", "attributes": null, "metadata": null}
								],
								"children": []
							}
						]
					}
					""";

			Footnote footnote = mapper.readValue(json, Footnote.class);

			assertNotNull(footnote);
			assertEquals("fn1", footnote.getLabel());

			List<FormattedBlock> content = footnote.getContent();
			assertEquals(1, content.size());

			// This would throw ClassCastException if elements were LinkedHashMap
			FormattedBlock block = content.get(0);
			assertEquals(BlockType.PARAGRAPH, block.getBlockType());
			assertEquals("Footnote text", block.getInlineContent().get(0).getContent());
		}
	}

	@Nested
	@DisplayName("Attributes deserialization")
	class AttributesTests {

		@Test
		@DisplayName("should deserialize List<KeyValue> correctly")
		void testKeyValuesDeserialization() throws Exception {
			String json = """
					{
						"id": "my-element",
						"classes": ["class1", "class2"],
						"key_values": [
							{"0": "data-foo", "1": "bar"},
							{"0": "data-baz", "1": "qux"}
						]
					}
					""";

			Attributes attrs = mapper.readValue(json, Attributes.class);

			assertNotNull(attrs);
			assertEquals("my-element", attrs.getId().orElse(null));
			assertEquals(List.of("class1", "class2"), attrs.getClasses());

			List<Attributes.KeyValue> keyValues = attrs.getKeyValues();
			assertEquals(2, keyValues.size());

			// This would throw ClassCastException if elements were LinkedHashMap
			Attributes.KeyValue first = keyValues.get(0);
			assertEquals("data-foo", first.getKey());
			assertEquals("bar", first.getValue());

			Attributes.KeyValue second = keyValues.get(1);
			assertEquals("data-baz", second.getKey());
			assertEquals("qux", second.getValue());
		}
	}

	@Nested
	@DisplayName("PageHierarchy deserialization")
	class PageHierarchyTests {

		@Test
		@DisplayName("should deserialize List<HierarchicalBlock> correctly")
		void testBlocksDeserialization() throws Exception {
			String json = """
					{
						"block_count": 2,
						"blocks": [
							{"text": "Chapter 1", "font_size": 24.0, "level": "h1", "bbox": [0, 0, 100, 30]},
							{"text": "Section 1.1", "font_size": 18.0, "level": "h2", "bbox": [0, 40, 100, 60]}
						]
					}
					""";

			PageHierarchy hierarchy = mapper.readValue(json, PageHierarchy.class);

			assertNotNull(hierarchy);
			assertEquals(2, hierarchy.blockCount());

			List<HierarchicalBlock> blocks = hierarchy.blocks();
			assertEquals(2, blocks.size());

			// This would throw ClassCastException if elements were LinkedHashMap
			HierarchicalBlock first = blocks.get(0);
			assertEquals("Chapter 1", first.text());
			assertEquals(24.0f, first.fontSize(), 0.01);
			assertEquals("h1", first.level());

			HierarchicalBlock second = blocks.get(1);
			assertEquals("Section 1.1", second.text());
			assertEquals(18.0f, second.fontSize(), 0.01);
			assertEquals("h2", second.level());
		}
	}

	@Nested
	@DisplayName("PageContent deserialization")
	class PageContentTests {

		@Test
		@DisplayName("should deserialize List<Table> correctly")
		void testTablesDeserialization() throws Exception {
			String json = """
					{
						"page_number": 1,
						"content": "Page content",
						"tables": [
							{"cells": [["A", "B"], ["1", "2"]], "markdown": "| A | B |\\n|---|---|\\n| 1 | 2 |", "page_number": 1}
						],
						"images": [],
						"hierarchy": null
					}
					""";

			PageContent pageContent = mapper.readValue(json, PageContent.class);

			assertNotNull(pageContent);
			assertEquals(1, pageContent.pageNumber());

			List<Table> tables = pageContent.tables();
			assertEquals(1, tables.size());

			// This would throw ClassCastException if elements were LinkedHashMap
			Table table = tables.get(0);
			assertEquals(2, table.getRowCount());
			assertEquals(2, table.getColumnCount());
			assertEquals("A", table.getCell(0, 0));
		}

		@Test
		@DisplayName("should deserialize List<ExtractedImage> correctly")
		void testImagesDeserialization() throws Exception {
			// ExtractedImage requires non-null data, so we provide base64 encoded empty PNG
			String json = """
					{
						"page_number": 2,
						"content": "Page with images",
						"tables": [],
						"images": [
							{
								"data": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
								"format": "png",
								"image_index": 0,
								"page_number": 2,
								"width": 800,
								"height": 600,
								"colorspace": "RGB",
								"bits_per_component": 8,
								"is_mask": false,
								"description": "Test image",
								"ocr_result": null
							}
						],
						"hierarchy": null
					}
					""";

			PageContent pageContent = mapper.readValue(json, PageContent.class);

			List<ExtractedImage> images = pageContent.images();
			assertEquals(1, images.size());

			// This would throw ClassCastException if elements were LinkedHashMap
			ExtractedImage image = images.get(0);
			assertEquals("png", image.getFormat());
			assertEquals(0, image.getImageIndex());
			assertEquals(800, image.getWidth().orElse(0));
			assertEquals(600, image.getHeight().orElse(0));
		}

		@Test
		@DisplayName("should deserialize PageHierarchy with nested blocks")
		void testHierarchyDeserialization() throws Exception {
			String json = """
					{
						"page_number": 1,
						"content": "Page content",
						"tables": [],
						"images": [],
						"hierarchy": {
							"block_count": 1,
							"blocks": [
								{"text": "Title", "font_size": 20.0, "level": "h1", "bbox": [0, 0, 100, 25]}
							]
						}
					}
					""";

			PageContent pageContent = mapper.readValue(json, PageContent.class);

			assertTrue(pageContent.getHierarchy().isPresent());
			PageHierarchy hierarchy = pageContent.getHierarchy().get();

			HierarchicalBlock block = hierarchy.blocks().get(0);
			assertEquals("Title", block.text());
		}
	}

	@Nested
	@DisplayName("DjotContent deserialization")
	class DjotContentTests {

		@Test
		@DisplayName("should deserialize all list fields correctly")
		void testFullDjotContentDeserialization() throws Exception {
			String json = """
					{
						"plain_text": "Hello world",
						"blocks": [
							{
								"block_type": "paragraph",
								"inline_content": [
									{"element_type": "text", "content": "Hello world", "attributes": null, "metadata": null}
								],
								"children": []
							}
						],
						"metadata": null,
						"tables": [
							{"cells": [["X"]], "markdown": "| X |", "page_number": 0}
						],
						"images": [
							{"src": "img.png", "alt": "Image", "title": null, "attributes": null}
						],
						"links": [
							{"url": "https://example.com", "text": "Example", "title": null, "attributes": null}
						],
						"footnotes": [
							{
								"label": "1",
								"content": [
									{
										"block_type": "paragraph",
										"inline_content": [
											{"element_type": "text", "content": "Note", "attributes": null, "metadata": null}
										],
										"children": []
									}
								]
							}
						],
						"attributes": [
							{"0": "elem1", "1": {"id": "id1", "classes": [], "key_values": []}}
						]
					}
					""";

			DjotContent content = mapper.readValue(json, DjotContent.class);

			assertNotNull(content);
			assertEquals("Hello world", content.getPlainText());

			// Test blocks
			List<FormattedBlock> blocks = content.getBlocks();
			assertEquals(1, blocks.size());
			FormattedBlock block = blocks.get(0);
			assertEquals(BlockType.PARAGRAPH, block.getBlockType());

			// Test tables
			List<Table> tables = content.getTables();
			assertEquals(1, tables.size());
			Table table = tables.get(0);
			assertEquals("X", table.getCell(0, 0));

			// Test images
			List<DjotImage> images = content.getImages();
			assertEquals(1, images.size());
			DjotImage image = images.get(0);
			assertEquals("img.png", image.getSrc());

			// Test links
			List<DjotLink> links = content.getLinks();
			assertEquals(1, links.size());
			DjotLink link = links.get(0);
			assertEquals("https://example.com", link.getUrl());

			// Test footnotes
			List<Footnote> footnotes = content.getFootnotes();
			assertEquals(1, footnotes.size());
			Footnote footnote = footnotes.get(0);
			assertEquals("1", footnote.getLabel());

			// Test attributes
			List<DjotContent.AttributeEntry> attributes = content.getAttributes();
			assertEquals(1, attributes.size());
			DjotContent.AttributeEntry attr = attributes.get(0);
			assertEquals("elem1", attr.getKey());
			assertEquals("id1", attr.getAttributes().getId().orElse(null));
		}

		@Test
		@DisplayName("should handle empty lists")
		void testEmptyLists() throws Exception {
			String json = """
					{
						"plain_text": "Empty doc",
						"blocks": [],
						"metadata": null,
						"tables": [],
						"images": [],
						"links": [],
						"footnotes": [],
						"attributes": []
					}
					""";

			DjotContent content = mapper.readValue(json, DjotContent.class);

			assertTrue(content.getBlocks().isEmpty());
			assertTrue(content.getTables().isEmpty());
			assertTrue(content.getImages().isEmpty());
			assertTrue(content.getLinks().isEmpty());
			assertTrue(content.getFootnotes().isEmpty());
			assertTrue(content.getAttributes().isEmpty());
		}
	}

	@Nested
	@DisplayName("PageStructure deserialization")
	class PageStructureTests {

		@Test
		@DisplayName("should deserialize List<PageBoundary> correctly")
		void testBoundariesDeserialization() throws Exception {
			String json = """
					{
						"total_count": 3,
						"unit_type": "page",
						"boundaries": [
							{"byte_start": 0, "byte_end": 500, "page_number": 1},
							{"byte_start": 500, "byte_end": 1000, "page_number": 2},
							{"byte_start": 1000, "byte_end": 1500, "page_number": 3}
						],
						"pages": null
					}
					""";

			PageStructure structure = mapper.readValue(json, new TypeReference<PageStructure>() {
			});

			assertEquals(3L, structure.getTotalCount());

			List<PageBoundary> boundaries = structure.getBoundaries().orElseThrow();
			assertEquals(3, boundaries.size());

			PageBoundary first = boundaries.get(0);
			assertEquals(0L, first.byteStart());
			assertEquals(500L, first.byteEnd());
			assertEquals(1L, first.pageNumber());
		}

		@Test
		@DisplayName("should deserialize List<PageInfo> correctly")
		void testPagesDeserialization() throws Exception {
			String json = """
					{
						"total_count": 2,
						"unit_type": "slide",
						"boundaries": null,
						"pages": [
							{"number": 1, "title": "Intro", "dimensions": [1920.0, 1080.0], "hidden": false},
							{"number": 2, "title": "Content", "dimensions": [1920.0, 1080.0], "hidden": true}
						]
					}
					""";

			PageStructure structure = mapper.readValue(json, new TypeReference<PageStructure>() {
			});

			assertEquals(PageUnitType.SLIDE, structure.getUnitType());

			List<PageInfo> pages = structure.getPages().orElseThrow();
			assertEquals(2, pages.size());

			PageInfo first = pages.get(0);
			assertEquals(1L, first.getNumber());
			assertEquals("Intro", first.getTitle().orElse(null));
			assertTrue(first.isVisible().orElse(false));

			PageInfo second = pages.get(1);
			assertEquals(2L, second.getNumber());
			assertFalse(second.isVisible().orElse(true));
		}
	}

	@Nested
	@DisplayName("Chunk deserialization")
	class ChunkTests {

		@Test
		@DisplayName("should deserialize List<Float> embedding correctly")
		void testEmbeddingDeserialization() throws Exception {
			String json = """
					{
						"content": "Sample text chunk",
						"embedding": [0.1, 0.2, 0.3, 0.4, 0.5],
						"metadata": {
							"byte_start": 0,
							"byte_end": 100,
							"first_page": 1,
							"last_page": 1,
							"token_count": 10,
							"chunk_index": 0,
							"total_chunks": 1
						}
					}
					""";

			Chunk chunk = mapper.readValue(json, Chunk.class);

			assertNotNull(chunk);
			assertEquals("Sample text chunk", chunk.getContent());

			List<Float> embedding = chunk.getEmbedding().orElseThrow();
			assertEquals(5, embedding.size());

			// Verify elements are Float, not LinkedHashMap
			assertEquals(0.1f, embedding.get(0), 0.001);
			assertEquals(0.5f, embedding.get(4), 0.001);
		}
	}

	@Nested
	@DisplayName("Table deserialization")
	class TableTests {

		@Test
		@DisplayName("should deserialize nested List<List<String>> cells correctly")
		void testCellsDeserialization() throws Exception {
			String json = """
					{
						"cells": [
							["Header 1", "Header 2", "Header 3"],
							["Row 1 Col 1", "Row 1 Col 2", "Row 1 Col 3"],
							["Row 2 Col 1", "Row 2 Col 2", "Row 2 Col 3"]
						],
						"markdown": "| Header 1 | Header 2 | Header 3 |",
						"page_number": 1
					}
					""";

			Table table = mapper.readValue(json, Table.class);

			assertNotNull(table);
			assertEquals(3, table.getRowCount());
			assertEquals(3, table.getColumnCount());

			// Verify nested list elements are Strings, not LinkedHashMap
			assertEquals("Header 1", table.getCell(0, 0));
			assertEquals("Row 1 Col 2", table.getCell(1, 1));
			assertEquals("Row 2 Col 3", table.getCell(2, 2));

			List<String> row = table.getRow(0);
			assertEquals("Header 2", row.get(1));
		}
	}

	@Nested
	@DisplayName("LayoutRegion deserialization")
	class LayoutRegionTests {

		@Test
		@DisplayName("should deserialize List<LayoutRegion> on PageContent correctly")
		void testLayoutRegionsDeserialization() throws Exception {
			String json = """
					{
						"page_number": 1,
						"content": "Page with layout",
						"tables": [],
						"images": [],
						"hierarchy": null,
						"layout_regions": [
							{
								"class": "picture",
								"confidence": 0.95,
								"bounding_box": {"x0": 0.1, "y0": 0.2, "x1": 0.5, "y1": 0.7},
								"area_fraction": 0.24
							},
							{
								"class": "text",
								"confidence": 0.87,
								"bounding_box": {"x0": 0.0, "y0": 0.0, "x1": 1.0, "y1": 0.15},
								"area_fraction": 0.15
							}
						]
					}
					""";

			PageContent pageContent = mapper.readValue(json, PageContent.class);

			assertNotNull(pageContent);
			assertTrue(pageContent.getLayoutRegions().isPresent());

			List<LayoutRegion> regions = pageContent.getLayoutRegions().get();
			assertEquals(2, regions.size());

			// This would throw ClassCastException if elements were LinkedHashMap
			LayoutRegion first = regions.get(0);
			assertEquals("picture", first.getClassName());
			assertEquals(0.95, first.getConfidence(), 1e-9);
			assertEquals(0.24, first.getAreaFraction(), 1e-9);
			assertNotNull(first.getBoundingBox());
			assertEquals(0.1, first.getBoundingBox().getX0(), 1e-9);
			assertEquals(0.5, first.getBoundingBox().getX1(), 1e-9);

			LayoutRegion second = regions.get(1);
			assertEquals("text", second.getClassName());
			assertEquals(0.87, second.getConfidence(), 1e-9);
		}

		@Test
		@DisplayName("should return empty Optional when layout_regions absent")
		void testMissingLayoutRegions() throws Exception {
			String json = """
					{
						"page_number": 2,
						"content": "No layout",
						"tables": [],
						"images": [],
						"hierarchy": null
					}
					""";

			PageContent pageContent = mapper.readValue(json, PageContent.class);

			assertFalse(pageContent.getLayoutRegions().isPresent());
		}

		@Test
		@DisplayName("should return empty Optional when layout_regions is empty list")
		void testEmptyLayoutRegions() throws Exception {
			String json = """
					{
						"page_number": 3,
						"content": "Empty layout",
						"tables": [],
						"images": [],
						"hierarchy": null,
						"layout_regions": []
					}
					""";

			PageContent pageContent = mapper.readValue(json, PageContent.class);

			assertFalse(pageContent.getLayoutRegions().isPresent());
		}
	}

	@Nested
	@DisplayName("Complex nested structures")
	class ComplexNestedTests {

		@Test
		@DisplayName("should deserialize PageContent with all nested types")
		void testPageContentWithAllNestedTypes() throws Exception {
			String json = """
					{
						"page_number": 1,
						"content": "Page content with tables and hierarchy",
						"tables": [
							{"cells": [["A", "B"], ["1", "2"]], "markdown": "| A | B |", "page_number": 1}
						],
						"images": [],
						"hierarchy": {
							"block_count": 2,
							"blocks": [
								{"text": "Title", "font_size": 24.0, "level": "h1", "bbox": [0, 0, 100, 30]},
								{"text": "Subtitle", "font_size": 18.0, "level": "h2", "bbox": [0, 40, 100, 60]}
							]
						}
					}
					""";

			PageContent pageContent = mapper.readValue(json, PageContent.class);

			assertNotNull(pageContent);
			assertEquals(1, pageContent.pageNumber());

			// Verify tables
			Table table = pageContent.tables().get(0);
			assertEquals("A", table.getCell(0, 0));
			assertEquals("2", table.getCell(1, 1));

			// Verify hierarchy
			PageHierarchy hierarchy = pageContent.getHierarchy().orElseThrow();
			assertEquals(2, hierarchy.blockCount());

			HierarchicalBlock block1 = hierarchy.blocks().get(0);
			assertEquals("Title", block1.text());
			assertEquals("h1", block1.level());

			HierarchicalBlock block2 = hierarchy.blocks().get(1);
			assertEquals("Subtitle", block2.text());
			assertEquals("h2", block2.level());
		}

		@Test
		@DisplayName("should deserialize deeply nested DjotContent")
		void testDeeplyNestedDjotContent() throws Exception {
			String json = """
					{
						"plain_text": "Complex document",
						"blocks": [
							{
								"block_type": "blockquote",
								"inline_content": [],
								"children": [
									{
										"block_type": "ordered_list",
										"inline_content": [],
										"children": [
											{
												"block_type": "list_item",
												"inline_content": [
													{"element_type": "text", "content": "Item 1", "attributes": null, "metadata": null},
													{"element_type": "emphasis", "content": "emphasized", "attributes": null, "metadata": null}
												],
												"children": []
											}
										]
									}
								]
							}
						],
						"metadata": null,
						"tables": [],
						"images": [],
						"links": [],
						"footnotes": [],
						"attributes": []
					}
					""";

			DjotContent content = mapper.readValue(json, DjotContent.class);

			FormattedBlock blockquote = content.getBlocks().get(0);
			assertEquals(BlockType.BLOCKQUOTE, blockquote.getBlockType());

			FormattedBlock orderedList = blockquote.getChildren().get(0);
			assertEquals(BlockType.ORDERED_LIST, orderedList.getBlockType());

			FormattedBlock listItem = orderedList.getChildren().get(0);
			assertEquals(BlockType.LIST_ITEM, listItem.getBlockType());

			List<InlineElement> inlineContent = listItem.getInlineContent();
			assertEquals(2, inlineContent.size());
			assertEquals("Item 1", inlineContent.get(0).getContent());
			assertEquals(InlineType.EMPHASIS, inlineContent.get(1).getElementType());
		}
	}
}
