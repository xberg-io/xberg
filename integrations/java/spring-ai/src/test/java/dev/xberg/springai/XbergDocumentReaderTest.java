package dev.xberg.springai;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;

import dev.xberg.ExtractionResult;
import dev.xberg.ExtractionResultFactory;
import dev.xberg.Xberg;
import dev.xberg.XbergException;
import dev.xberg.config.ExtractionConfig;
import java.io.IOException;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;
import org.mockito.MockedStatic;
import org.mockito.Mockito;
import org.springframework.ai.document.Document;
import org.springframework.core.io.ByteArrayResource;
import org.springframework.core.io.ClassPathResource;
import org.springframework.core.io.FileSystemResource;
import org.springframework.core.io.Resource;

class XbergDocumentReaderTest {

	private static ExtractionResult createResult(String json) throws Exception {
		return ExtractionResultFactory.fromJson(json);
	}

	// -- 1. Builder Validation ------------------------------------------------

	@Nested
	class BuilderValidation {

		@Test
		void shouldThrowWhenResourceIsNull() {
			assertThatThrownBy(() -> XbergDocumentReader.builder().build())
					.isInstanceOf(IllegalArgumentException.class).hasMessageContaining("resource is required");
		}

		@Test
		void shouldThrowWhenByteArrayResourceWithoutMimeType() {
			ByteArrayResource resource = new ByteArrayResource(new byte[]{1, 2, 3});
			assertThatThrownBy(() -> XbergDocumentReader.builder().resource(resource).build())
					.isInstanceOf(IllegalArgumentException.class).hasMessageContaining("mimeType is required");
		}

		@Test
		void shouldBuildWithFileSystemResource() {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			XbergDocumentReader reader = XbergDocumentReader.builder().resource(resource).build();
			assertThat(reader).isNotNull();
		}

		@Test
		void shouldBuildWithByteArrayResourceAndMimeType() {
			ByteArrayResource resource = new ByteArrayResource(new byte[]{1, 2, 3});
			XbergDocumentReader reader = XbergDocumentReader.builder().resource(resource)
					.mimeType("application/pdf").build();
			assertThat(reader).isNotNull();
		}
	}

	// -- 2. Resource Extraction via FileSystemResource ------------------------

	@Nested
	class FileSystemResourceExtraction {

		@Test
		void shouldExtractFromFileSystemResource() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{"content":"Hello","mime_type":"application/pdf","metadata":{},\
					"tables":[],"detected_languages":["en"],"chunks":[],"images":[],\
					"pages":[],"elements":[]}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				assertThat(docs).hasSize(1);
				assertThat(docs.getFirst().getText()).isEqualTo("Hello");
				mocked.verify(() -> Xberg.extractFile(any(Path.class)));
			}
		}

		@Test
		void shouldPassExtractionConfigToExtractFile() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionConfig config = ExtractionConfig.builder().build();
			ExtractionResult result = createResult("""
					{"content":"Hello","mime_type":"application/pdf","metadata":{},\
					"tables":[],"detected_languages":[],"chunks":[],"images":[],\
					"pages":[],"elements":[]}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class), any(ExtractionConfig.class)))
						.thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).extractionConfig(config)
						.build().get();

				assertThat(docs).hasSize(1);
				mocked.verify(() -> Xberg.extractFile(any(Path.class), any(ExtractionConfig.class)));
			}
		}
	}

	// -- 3. Resource Extraction via ByteArrayResource -------------------------

	@Nested
	class ByteArrayResourceExtraction {

		@Test
		void shouldExtractFromByteArrayResource() throws Exception {
			byte[] data = new byte[]{1, 2, 3};
			ByteArrayResource resource = new ByteArrayResource(data);
			ExtractionResult result = createResult("""
					{"content":"Bytes content","mime_type":"application/pdf","metadata":{},\
					"tables":[],"detected_languages":[],"chunks":[],"images":[],\
					"pages":[],"elements":[]}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(
						() -> Xberg.extractBytes(any(byte[].class), any(String.class), any(ExtractionConfig.class)))
						.thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).mimeType("application/pdf")
						.build().get();

				assertThat(docs).hasSize(1);
				assertThat(docs.getFirst().getText()).isEqualTo("Bytes content");
				mocked.verify(() -> Xberg.extractBytes(any(byte[].class), eq("application/pdf"),
						any(ExtractionConfig.class)));
			}
		}

		@Test
		void shouldPassMimeTypeToExtractBytes() throws Exception {
			byte[] data = new byte[]{4, 5, 6};
			ByteArrayResource resource = new ByteArrayResource(data);
			ExtractionResult result = createResult(
					"""
							{"content":"DOCX content","mime_type":"application/vnd.openxmlformats-officedocument.wordprocessingml.document",\
							"metadata":{},"tables":[],"detected_languages":[],"chunks":[],"images":[],\
							"pages":[],"elements":[]}""");

			String docxMime = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(
						() -> Xberg.extractBytes(any(byte[].class), any(String.class), any(ExtractionConfig.class)))
						.thenReturn(result);

				XbergDocumentReader.builder().resource(resource).mimeType(docxMime).build().get();

				mocked.verify(
						() -> Xberg.extractBytes(any(byte[].class), eq(docxMime), any(ExtractionConfig.class)));
			}
		}
	}

	// -- 4. Resource Extraction via ClassPathResource -------------------------

	@Nested
	class ClassPathResourceExtraction {

		@Test
		void shouldExtractFromClassPathResource() throws Exception {
			ClassPathResource resource = new ClassPathResource("fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{"content":"ClassPath content","mime_type":"application/pdf","metadata":{},\
					"tables":[],"detected_languages":[],"chunks":[],"images":[],\
					"pages":[],"elements":[]}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(
						() -> Xberg.extractBytes(any(byte[].class), any(String.class), any(ExtractionConfig.class)))
						.thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				assertThat(docs).hasSize(1);
				assertThat(docs.getFirst().getText()).isEqualTo("ClassPath content");
				mocked.verify(() -> Xberg.extractBytes(any(byte[].class), any(String.class),
						any(ExtractionConfig.class)));
			}
		}

		@Test
		void shouldUseExplicitMimeTypeOverFilename() throws Exception {
			ClassPathResource resource = new ClassPathResource("fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{"content":"Override content","mime_type":"text/plain","metadata":{},\
					"tables":[],"detected_languages":[],"chunks":[],"images":[],\
					"pages":[],"elements":[]}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(
						() -> Xberg.extractBytes(any(byte[].class), any(String.class), any(ExtractionConfig.class)))
						.thenReturn(result);

				XbergDocumentReader.builder().resource(resource).mimeType("text/plain").build().get();

				mocked.verify(
						() -> Xberg.extractBytes(any(byte[].class), eq("text/plain"), any(ExtractionConfig.class)));
			}
		}
	}

	// -- 5. Base Metadata Mapping ---------------------------------------------

	@Nested
	class BaseMetadataMapping {

		@Test
		void shouldMapAllExplicitMetadataFields() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Text",
					  "mime_type": "application/pdf",
					  "metadata": {
					    "title": "Test Document",
					    "subject": "Testing",
					    "authors": ["John Doe", "Jane Smith"],
					    "keywords": ["test", "document"],
					    "language": "en",
					    "created_at": "2025-01-01T00:00:00Z",
					    "modified_at": "2025-06-01T00:00:00Z",
					    "created_by": "TestUser",
					    "modified_by": "TestEditor",
					    "category": "Testing",
					    "tags": ["unit-test", "integration"],
					    "document_version": "1.0",
					    "abstract_text": "A test document abstract",
					    "output_format": "plain"
					  },
					  "tables": [{"cells": [["A","B"]], "markdown": "| A | B |", "page_number": 0}],
					  "detected_languages": ["en", "de"],
					  "quality_score": 0.95,
					  "chunks": [],
					  "images": [],
					  "pages": [],
					  "elements": [],
					  "extracted_keywords": [{"text": "test", "score": 0.9, "algorithm": "yake"}],
					  "processing_warnings": [{"source": "ocr", "message": "Low confidence"}]
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				Map<String, Object> metadata = docs.getFirst().getMetadata();
				assertThat(metadata.get("source")).isEqualTo("sample.pdf");
				assertThat(metadata.get("mime_type")).isEqualTo("application/pdf");
				assertThat(metadata.get("title")).isEqualTo("Test Document");
				assertThat(metadata.get("subject")).isEqualTo("Testing");
				assertThat(metadata.get("authors")).isEqualTo("John Doe, Jane Smith");
				assertThat(metadata.get("keywords")).isEqualTo("test, document");
				assertThat(metadata.get("language")).isEqualTo("en");
				assertThat(metadata.get("created_at")).isEqualTo("2025-01-01T00:00:00Z");
				assertThat(metadata.get("modified_at")).isEqualTo("2025-06-01T00:00:00Z");
				assertThat(metadata.get("created_by")).isEqualTo("TestUser");
				assertThat(metadata.get("modified_by")).isEqualTo("TestEditor");
				assertThat(metadata.get("category")).isEqualTo("Testing");
				assertThat(metadata.get("tags")).isEqualTo("unit-test, integration");
				assertThat(metadata.get("document_version")).isEqualTo("1.0");
				assertThat(metadata.get("abstract_text")).isEqualTo("A test document abstract");
				assertThat(metadata.get("output_format")).isEqualTo("plain");
				assertThat(metadata.get("detected_languages")).isEqualTo("en, de");
				assertThat(metadata.get("quality_score")).isEqualTo(0.95);
				assertThat(metadata.get("table_count")).isEqualTo(1);
				assertThat(metadata).containsKey("tables");
				assertThat(metadata).containsKey("extracted_keywords");
				assertThat(metadata).containsKey("processing_warnings");
			}
		}

		@Test
		void shouldMapMinimalMetadata() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{"content":"Minimal","mime_type":"application/pdf","metadata":{},\
					"tables":[],"detected_languages":["en"],"chunks":[],"images":[],\
					"pages":[],"elements":[]}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				Map<String, Object> metadata = docs.getFirst().getMetadata();
				assertThat(metadata).containsKey("source");
				assertThat(metadata).containsKey("mime_type");
				assertThat(metadata).containsKey("page_count");
				assertThat(metadata).containsKey("detected_languages");
				assertThat(metadata).containsKey("table_count");
				assertThat(metadata).doesNotContainKey("title");
				assertThat(metadata).doesNotContainKey("subject");
				assertThat(metadata).doesNotContainKey("authors");
			}
		}

		@Test
		void shouldJoinListMetadata() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Text",
					  "mime_type": "application/pdf",
					  "metadata": {
					    "authors": ["John", "Jane"],
					    "keywords": ["a", "b", "c"],
					    "tags": ["x", "y"]
					  },
					  "tables": [],
					  "detected_languages": ["en", "de"],
					  "chunks": [],
					  "images": [],
					  "pages": [],
					  "elements": []
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				Map<String, Object> metadata = docs.getFirst().getMetadata();
				assertThat(metadata.get("authors")).isEqualTo("John, Jane");
				assertThat(metadata.get("keywords")).isEqualTo("a, b, c");
				assertThat(metadata.get("tags")).isEqualTo("x, y");
				assertThat(metadata.get("detected_languages")).isEqualTo("en, de");
			}
		}

		@Test
		void shouldMergeUserMetadata() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Text",
					  "mime_type": "application/pdf",
					  "metadata": {"title": "Original Title"},
					  "tables": [],
					  "detected_languages": [],
					  "chunks": [],
					  "images": [],
					  "pages": [],
					  "elements": []
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource)
						.metadata("title", "User Title").metadata("custom_key", "custom_value").build().get();

				Map<String, Object> metadata = docs.getFirst().getMetadata();
				assertThat(metadata.get("title")).isEqualTo("User Title");
				assertThat(metadata.get("custom_key")).isEqualTo("custom_value");
			}
		}
	}

	// -- 6. Metadata Serialization --------------------------------------------

	@Nested
	class MetadataSerialization {

		@Test
		void shouldSerializeTablesToJson() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Text",
					  "mime_type": "application/pdf",
					  "metadata": {},
					  "tables": [{"cells": [["A","B"]], "markdown": "| A | B |", "page_number": 0}],
					  "detected_languages": [],
					  "chunks": [],
					  "images": [],
					  "pages": [],
					  "elements": []
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				Map<String, Object> metadata = docs.getFirst().getMetadata();
				assertThat(metadata.get("table_count")).isEqualTo(1);
				assertThat(metadata.get("tables")).isInstanceOf(String.class);
				String tablesJson = (String) metadata.get("tables");
				assertThat(tablesJson).contains("| A | B |");
			}
		}

		@Test
		void shouldPassThroughFormatSpecificMetadata() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Text",
					  "mime_type": "application/pdf",
					  "metadata": {
					    "pdf_version": "1.7",
					    "producer": "TestProducer",
					    "custom_list": ["a", "b"]
					  },
					  "tables": [],
					  "detected_languages": [],
					  "chunks": [],
					  "images": [],
					  "pages": [],
					  "elements": []
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				Map<String, Object> metadata = docs.getFirst().getMetadata();
				assertThat(metadata.get("pdf_version")).isEqualTo("1.7");
				assertThat(metadata.get("producer")).isEqualTo("TestProducer");
				assertThat(metadata.get("custom_list")).isInstanceOf(String.class);
			}
		}

		@Test
		void shouldLetExplicitFieldsOverrideAdditional() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Text",
					  "mime_type": "application/pdf",
					  "metadata": {
					    "title": "Typed Title",
					    "subject": "Typed Subject"
					  },
					  "tables": [],
					  "detected_languages": [],
					  "chunks": [],
					  "images": [],
					  "pages": [],
					  "elements": []
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				Map<String, Object> metadata = docs.getFirst().getMetadata();
				assertThat(metadata.get("title")).isEqualTo("Typed Title");
				assertThat(metadata.get("subject")).isEqualTo("Typed Subject");
			}
		}
	}

	// -- 7. Chunk-Based Splitting ---------------------------------------------

	@Nested
	class ChunkBasedSplitting {

		@Test
		void shouldCreateDocumentsFromChunks() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Full text",
					  "mime_type": "application/pdf",
					  "metadata": {},
					  "tables": [],
					  "detected_languages": [],
					  "chunks": [
					    {
					      "content": "Chunk 1 text",
					      "metadata": {"chunk_index": 0, "total_chunks": 2, "byte_start": 0, "byte_end": 100}
					    },
					    {
					      "content": "Chunk 2 text",
					      "metadata": {"chunk_index": 1, "total_chunks": 2, "byte_start": 100, "byte_end": 200}
					    }
					  ],
					  "images": [],
					  "pages": [],
					  "elements": []
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				assertThat(docs).hasSize(2);
				assertThat(docs.get(0).getText()).isEqualTo("Chunk 1 text");
				assertThat(docs.get(1).getText()).isEqualTo("Chunk 2 text");
			}
		}

		@Test
		void shouldPopulateChunkMetadata() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Full text",
					  "mime_type": "application/pdf",
					  "metadata": {},
					  "tables": [],
					  "detected_languages": [],
					  "chunks": [
					    {
					      "content": "Chunk text",
					      "metadata": {
					        "chunk_index": 0,
					        "total_chunks": 1,
					        "byte_start": 0,
					        "byte_end": 50,
					        "token_count": 10,
					        "first_page": 1,
					        "last_page": 2,
					        "heading_context": {
					          "headings": [{"level": 1, "text": "Introduction"}]
					        }
					      }
					    }
					  ],
					  "images": [],
					  "pages": [],
					  "elements": []
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				assertThat(docs).hasSize(1);
				Map<String, Object> metadata = docs.getFirst().getMetadata();
				assertThat(metadata.get("chunk_index")).isEqualTo(0);
				assertThat(metadata.get("total_chunks")).isEqualTo(1);
				assertThat(metadata.get("token_count")).isEqualTo(10);
				assertThat(metadata.get("first_page")).isEqualTo(1L);
				assertThat(metadata.get("last_page")).isEqualTo(2L);
				assertThat(metadata.get("heading_context")).isInstanceOf(String.class);
				assertThat((String) metadata.get("heading_context")).contains("Introduction");
			}
		}
	}

	// -- 8. Element-Based Splitting -------------------------------------------

	@Nested
	class ElementBasedSplitting {

		@Test
		void shouldCreateDocumentsFromElements() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Title text\\nParagraph text",
					  "mime_type": "application/pdf",
					  "metadata": {},
					  "tables": [],
					  "detected_languages": [],
					  "chunks": [],
					  "images": [],
					  "pages": [],
					  "elements": [
					    {
					      "element_id": "elem-1",
					      "element_type": "title",
					      "text": "Title text",
					      "metadata": {"page_number": 1, "element_index": 0}
					    },
					    {
					      "element_id": "elem-2",
					      "element_type": "narrative_text",
					      "text": "Paragraph text",
					      "metadata": {"page_number": 1, "element_index": 1}
					    }
					  ]
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				assertThat(docs).hasSize(2);
				assertThat(docs.get(0).getText()).isEqualTo("Title text");
				assertThat(docs.get(1).getText()).isEqualTo("Paragraph text");
			}
		}

		@Test
		void shouldPopulateElementMetadata() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Title text",
					  "mime_type": "application/pdf",
					  "metadata": {},
					  "tables": [],
					  "detected_languages": [],
					  "chunks": [],
					  "images": [],
					  "pages": [],
					  "elements": [
					    {
					      "element_id": "elem-1",
					      "element_type": "title",
					      "text": "Title text",
					      "metadata": {
					        "page_number": 1,
					        "element_index": 0,
					        "coordinates": {"x0": 10.0, "y0": 20.0, "x1": 500.0, "y1": 50.0}
					      }
					    }
					  ]
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				assertThat(docs).hasSize(1);
				Map<String, Object> metadata = docs.getFirst().getMetadata();
				assertThat(metadata.get("element_id")).isEqualTo("elem-1");
				assertThat(metadata.get("element_type")).isEqualTo("title");
				assertThat(metadata.get("page_number")).isEqualTo(1);
				assertThat(metadata.get("element_index")).isEqualTo(0);
				assertThat(metadata.get("bbox_x0")).isEqualTo(10.0);
				assertThat(metadata.get("bbox_y0")).isEqualTo(20.0);
				assertThat(metadata.get("bbox_x1")).isEqualTo(500.0);
				assertThat(metadata.get("bbox_y1")).isEqualTo(50.0);
			}
		}
	}

	// -- 9. Page Splitting ----------------------------------------------------

	@Nested
	class PageSplitting {

		@Test
		void shouldCreateDocumentsFromPages() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Page 1 text\\nPage 2 text",
					  "mime_type": "application/pdf",
					  "metadata": {},
					  "tables": [],
					  "detected_languages": ["en"],
					  "chunks": [],
					  "images": [],
					  "pages": [
					    {"page_number": 1, "content": "Page 1 text", "tables": [], "images": []},
					    {"page_number": 2, "content": "Page 2 text", "tables": [], "images": []}
					  ],
					  "elements": []
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				assertThat(docs).hasSize(2);
				assertThat(docs.get(0).getText()).isEqualTo("Page 1 text");
				assertThat(docs.get(1).getText()).isEqualTo("Page 2 text");
				assertThat(docs.get(0).getMetadata().get("page")).isEqualTo(1);
				assertThat(docs.get(1).getMetadata().get("page")).isEqualTo(2);
			}
		}
	}

	// -- 10. Splitting Priority -----------------------------------------------

	@Nested
	class SplittingPriority {

		@Test
		void shouldPreferChunksOverPages() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Full text",
					  "mime_type": "application/pdf",
					  "metadata": {},
					  "tables": [],
					  "detected_languages": [],
					  "chunks": [
					    {
					      "content": "Chunk content",
					      "metadata": {"chunk_index": 0, "total_chunks": 1, "byte_start": 0, "byte_end": 50}
					    }
					  ],
					  "images": [],
					  "pages": [
					    {"page_number": 1, "content": "Page content", "tables": [], "images": []}
					  ],
					  "elements": []
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				assertThat(docs).hasSize(1);
				assertThat(docs.getFirst().getText()).isEqualTo("Chunk content");
			}
		}

		@Test
		void shouldPreferElementsOverPages() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{
					  "content": "Full text",
					  "mime_type": "application/pdf",
					  "metadata": {},
					  "tables": [],
					  "detected_languages": [],
					  "chunks": [],
					  "images": [],
					  "pages": [
					    {"page_number": 1, "content": "Page content", "tables": [], "images": []}
					  ],
					  "elements": [
					    {
					      "element_id": "e1",
					      "element_type": "narrative_text",
					      "text": "Element content",
					      "metadata": {"page_number": 1}
					    }
					  ]
					}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				assertThat(docs).hasSize(1);
				assertThat(docs.getFirst().getText()).isEqualTo("Element content");
			}
		}

		@Test
		void shouldReturnSingleDocumentWhenNothingPresent() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{"content":"Single content","mime_type":"application/pdf","metadata":{},\
					"tables":[],"detected_languages":[],"chunks":[],"images":[],\
					"pages":[],"elements":[]}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				assertThat(docs).hasSize(1);
				assertThat(docs.getFirst().getText()).isEqualTo("Single content");
			}
		}
	}

	// -- 11. Source Resolution ------------------------------------------------

	@Nested
	class SourceResolution {

		@Test
		void shouldResolveSourceFromFilename() throws Exception {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");
			ExtractionResult result = createResult("""
					{"content":"Text","mime_type":"application/pdf","metadata":{},\
					"tables":[],"detected_languages":[],"chunks":[],"images":[],\
					"pages":[],"elements":[]}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class))).thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).build().get();

				assertThat(docs.getFirst().getMetadata().get("source")).isEqualTo("sample.pdf");
			}
		}

		@Test
		void shouldResolveSourceForByteArrayResource() throws Exception {
			ByteArrayResource resource = new ByteArrayResource(new byte[]{1, 2, 3});
			ExtractionResult result = createResult("""
					{"content":"Text","mime_type":"application/pdf","metadata":{},\
					"tables":[],"detected_languages":[],"chunks":[],"images":[],\
					"pages":[],"elements":[]}""");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(
						() -> Xberg.extractBytes(any(byte[].class), any(String.class), any(ExtractionConfig.class)))
						.thenReturn(result);

				List<Document> docs = XbergDocumentReader.builder().resource(resource).mimeType("application/pdf")
						.build().get();

				assertThat(docs.getFirst().getMetadata().get("source")).isEqualTo("bytes://application/pdf");
			}
		}
	}

	// -- 12. Error Handling ---------------------------------------------------

	@Nested
	class ErrorHandling {

		@Test
		void shouldWrapXbergException() {
			FileSystemResource resource = new FileSystemResource("src/test/resources/fixtures/sample.pdf");

			try (MockedStatic<Xberg> mocked = Mockito.mockStatic(Xberg.class)) {
				mocked.when(() -> Xberg.extractFile(any(Path.class)))
						.thenThrow(new XbergException("extraction failed"));

				XbergDocumentReader reader = XbergDocumentReader.builder().resource(resource).build();

				assertThatThrownBy(reader::get).isInstanceOf(RuntimeException.class)
						.hasMessageContaining("Xberg extraction failed")
						.hasCauseInstanceOf(XbergException.class);
			}
		}

		@Test
		void shouldWrapIOException() throws Exception {
			Resource mockResource = Mockito.mock(Resource.class);
			Mockito.when(mockResource.getFilename()).thenReturn("test.pdf");
			Mockito.when(mockResource.getInputStream()).thenThrow(new IOException("read failed"));

			XbergDocumentReader reader = XbergDocumentReader.builder().resource(mockResource).build();

			assertThatThrownBy(reader::get).isInstanceOf(RuntimeException.class)
					.hasMessageContaining("Failed to extract document").hasCauseInstanceOf(IOException.class);
		}
	}
}
