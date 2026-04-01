# frozen_string_literal: true

require 'sorbet-runtime'

module Kreuzberg
  # Semantic element type classification.
  #
  # Categorizes text content into semantic units for downstream processing.
  # Supports the element types commonly found in Unstructured documents.
  #
  # @example
  #   type = Kreuzberg::ElementType::TITLE
  #   Kreuzberg::ElementType.values # => ["title", "narrative_text", ...]
  #
  module ElementType
    TITLE = 'title'
    NARRATIVE_TEXT = 'narrative_text'
    HEADING = 'heading'
    LIST_ITEM = 'list_item'
    TABLE = 'table'
    IMAGE = 'image'
    PAGE_BREAK = 'page_break'
    CODE_BLOCK = 'code_block'
    BLOCK_QUOTE = 'block_quote'
    FOOTER = 'footer'
    HEADER = 'header'

    def self.values
      [TITLE, NARRATIVE_TEXT, HEADING, LIST_ITEM, TABLE, IMAGE, PAGE_BREAK, CODE_BLOCK, BLOCK_QUOTE, FOOTER, HEADER]
    end
  end

  # Bounding box coordinates for element positioning.
  #
  # Represents rectangular coordinates for an element within a page.
  #
  # @example
  #   bbox = Kreuzberg::BoundingBox.new(
  #     x0: 10.0,
  #     y0: 20.0,
  #     x1: 100.0,
  #     y1: 50.0
  #   )
  #   puts "Width: #{bbox.x1 - bbox.x0}"
  #
  class BoundingBox < T::Struct
    extend T::Sig

    const :x0, Float

    const :y0, Float

    const :x1, Float

    const :y1, Float
  end

  # Metadata for a semantic element.
  #
  # Provides contextual information about an extracted element including
  # its position within the document and custom metadata fields.
  #
  # @example
  #   metadata = Kreuzberg::ElementMetadata.new(
  #     page_number: 1,
  #     filename: "document.pdf",
  #     coordinates: bbox,
  #     element_index: 5,
  #     additional: { "style" => "bold" }
  #   )
  #
  class ElementMetadata < T::Struct
    extend T::Sig

    const :page_number, T.nilable(Integer)

    const :filename, T.nilable(String)

    const :coordinates, T.nilable(BoundingBox)

    const :element_index, T.nilable(Integer)

    const :additional, T::Hash[String, String]
  end

  # Semantic element extracted from document.
  #
  # Represents a logical unit of content with semantic classification,
  # unique identifier, and metadata for tracking origin and position.
  # Compatible with Unstructured.io element format when output_format='element_based'.
  #
  # @example
  #   element = Kreuzberg::Element.new(
  #     element_id: "elem-abc123",
  #     element_type: "narrative_text",
  #     text: "This is the main content.",
  #     metadata: metadata
  #   )
  #   puts "#{element.element_type}: #{element.text}"
  #
  class Element < T::Struct
    extend T::Sig

    const :element_id, String

    const :element_type, String

    const :text, String

    const :metadata, ElementMetadata
  end

  # Header/Heading metadata
  #
  # Represents a heading element found in the HTML document
  #
  # @example
  #   header = Kreuzberg::HeaderMetadata.new(
  #     level: 1,
  #     text: "Main Title",
  #     id: "main-title",
  #     depth: 0,
  #     html_offset: 245
  #   )
  #   puts "#{header.text} (H#{header.level})"
  #
  class HeaderMetadata < T::Struct
    extend T::Sig

    const :level, Integer

    const :text, String

    const :id, T.nilable(String)

    const :depth, Integer

    const :html_offset, Integer
  end

  # Link metadata
  #
  # Represents a link element found in the HTML document
  #
  # @example
  #   link = Kreuzberg::LinkMetadata.new(
  #     href: "https://example.com",
  #     text: "Example",
  #     title: "Example Site",
  #     link_type: "external",
  #     rel: ["noopener", "noreferrer"],
  #     attributes: { "data-id" => "123" }
  #   )
  #   puts "#{link.text} -> #{link.href}"
  #
  class LinkMetadata < T::Struct
    extend T::Sig

    const :href, String

    const :text, String

    const :title, T.nilable(String)

    const :link_type, String

    const :rel, T::Array[String]

    const :attributes, T::Hash[String, String]
  end

  # Image metadata
  #
  # Represents an image element found in the HTML document
  #
  # @example
  #   image = Kreuzberg::ImageMetadata.new(
  #     src: "images/logo.png",
  #     alt: "Company Logo",
  #     title: nil,
  #     dimensions: [200, 100],
  #     image_type: "png",
  #     attributes: { "loading" => "lazy" }
  #   )
  #   if image.dimensions
  #     width, height = image.dimensions
  #     puts "#{width}x#{height}"
  #   end
  #
  class ImageMetadata < T::Struct
    extend T::Sig

    const :src, String

    const :alt, T.nilable(String)

    const :title, T.nilable(String)

    const :dimensions, T.nilable(T::Array[Integer])

    const :image_type, String

    const :attributes, T::Hash[String, String]
  end

  # Structured data metadata
  #
  # Represents structured data (JSON-LD, microdata, etc.) found in the HTML document
  #
  # @example
  #   structured = Kreuzberg::StructuredData.new(
  #     data_type: "json-ld",
  #     raw_json: '{"@context":"https://schema.org","@type":"Article",...}',
  #     schema_type: "Article"
  #   )
  #   data = JSON.parse(structured.raw_json)
  #   puts data['@type']
  #
  class StructuredData < T::Struct
    extend T::Sig

    const :data_type, String

    const :raw_json, String

    const :schema_type, T.nilable(String)
  end

  # @example
  class HtmlMetadata < T::Struct
    extend T::Sig

    const :title, T.nilable(String)

    const :description, T.nilable(String)

    const :author, T.nilable(String)

    const :copyright, T.nilable(String)

    const :keywords, T::Array[String]

    const :canonical_url, T.nilable(String)

    const :language, T.nilable(String)

    const :text_direction, T.nilable(String)

    const :mime_type, T.nilable(String)

    const :charset, T.nilable(String)

    const :generator, T.nilable(String)

    const :viewport, T.nilable(String)

    const :theme_color, T.nilable(String)

    const :application_name, T.nilable(String)

    const :robots, T.nilable(String)

    const :open_graph, T::Hash[String, String]

    const :twitter_card, T::Hash[String, String]

    const :meta_tags, T::Hash[String, String]

    const :headers, T::Array[HeaderMetadata]

    const :links, T::Array[LinkMetadata]

    const :images, T::Array[ImageMetadata]

    const :structured_data, T::Array[StructuredData]
  end

  # Extracted keyword with relevance metadata.
  #
  # Represents a single keyword extracted from text along with its relevance score,
  # the algorithm that extracted it, and optional position information.
  #
  # @example
  #   keyword = Kreuzberg::ExtractedKeyword.new(
  #     text: "machine learning",
  #     score: 0.95,
  #     algorithm: "yake",
  #     positions: [42, 128]
  #   )
  #   puts "#{keyword.text}: #{keyword.score}"
  #
  class ExtractedKeyword < T::Struct
    extend T::Sig

    const :text, String

    const :score, Float

    const :algorithm, String

    const :positions, T.nilable(T::Array[Integer])
  end

  # Processing warning from a pipeline stage.
  #
  # Represents a non-fatal warning generated during document processing.
  #
  # @example
  #   warning = Kreuzberg::ProcessingWarning.new(
  #     source: "ocr",
  #     message: "Low confidence on page 3"
  #   )
  #   puts "[#{warning.source}] #{warning.message}"
  #
  class ProcessingWarning < T::Struct
    extend T::Sig

    const :source, String

    const :message, String
  end

  # Bounding box for document node positioning.
  #
  # Represents rectangular coordinates for a node within the document.
  #
  # @example
  #   bbox = Kreuzberg::DocumentBoundingBox.new(
  #     x0: 10.0,
  #     y0: 20.0,
  #     x1: 100.0,
  #     y1: 50.0
  #   )
  #
  class DocumentBoundingBox < T::Struct
    extend T::Sig

    const :x0, Float

    const :y0, Float

    const :x1, Float

    const :y1, Float
  end

  # Annotation for a document node.
  #
  # Provides additional metadata about document node content.
  #
  class DocumentAnnotation < T::Struct
    extend T::Sig

    const :key, String

    const :value, String
  end

  # Single node in the document structure tree.
  #
  # Represents a logical unit of content with deterministic ID, content,
  # tree structure information, and metadata.
  #
  # @example
  #   node = Kreuzberg::DocumentNode.new(
  #     id: "node-abc123",
  #     content: "This is the content",
  #     parent: nil,
  #     children: [],
  #     content_layer: "body",
  #     page: 1,
  #     page_end: 1,
  #     bbox: bbox,
  #     annotations: []
  #   )
  #
  class DocumentNode < T::Struct
    extend T::Sig

    const :id, String

    const :content, String

    const :parent, T.nilable(Integer)

    const :children, T::Array[Integer]

    const :content_layer, String

    const :page, T.nilable(Integer)

    const :page_end, T.nilable(Integer)

    const :bbox, T.nilable(DocumentBoundingBox)

    const :annotations, T::Array[DocumentAnnotation]
  end

  # Structured document representation.
  #
  # Provides a hierarchical, tree-based representation of document content
  # using a flat array of nodes with index-based parent/child references.
  #
  # @example
  #   structure = Kreuzberg::DocumentStructure.new(
  #     nodes: [node1, node2, node3]
  #   )
  #   structure.nodes.each do |node|
  #     puts "#{node.id}: #{node.content}"
  #   end
  #
  class DocumentStructure < T::Struct
    extend T::Sig

    const :nodes, T::Array[DocumentNode]
  end

  # Bounding box for a PDF annotation.
  class PdfAnnotationBoundingBox < T::Struct
    extend T::Sig

    const :left, T.nilable(Float)
    const :top, T.nilable(Float)
    const :right, T.nilable(Float)
    const :bottom, T.nilable(Float)
  end

  # A PDF annotation extracted from a document page.
  class PdfAnnotation < T::Struct
    extend T::Sig

    const :annotation_type, String
    const :content, T.nilable(String)
    const :page_number, T.nilable(Integer)
    const :bounding_box, T.nilable(PdfAnnotationBoundingBox)
  end

  # An entry within an archive (zip, tar, etc.) extraction result.
  #
  # @example
  #   entry = Kreuzberg::ArchiveEntry.new(
  #     path: "readme.txt",
  #     mime_type: "text/plain",
  #     result: extraction_result
  #   )
  #
  class ArchiveEntry < T::Struct
    extend T::Sig

    const :path, String
    const :mime_type, String
    const :result, T.untyped
  end

  # Extracted keyword with relevance metadata.
  #
  # @example
  #   kw = Kreuzberg::Keyword.new(
  #     text: "machine learning",
  #     score: 0.95,
  #     algorithm: "yake",
  #     positions: [42, 128]
  #   )
  #
  class Keyword < T::Struct
    extend T::Sig

    const :text, String
    const :score, Float
    const :algorithm, String
    const :positions, T.nilable(T::Array[Integer])
  end

  # A table extracted from a document.
  #
  # @example
  #   table = Kreuzberg::Table.new(
  #     cells: [["A", "B"], ["1", "2"]],
  #     markdown: "| A | B |\n|---|---|\n| 1 | 2 |",
  #     page_number: 1,
  #     bounding_box: bbox
  #   )
  #
  class Table < T::Struct
    extend T::Sig

    const :cells, T::Array[T::Array[String]]
    const :markdown, String
    const :page_number, Integer
    const :bounding_box, T.nilable(BoundingBox)
  end

  # A URI extracted from a document.
  #
  # @example
  #   uri = Kreuzberg::Uri.new(
  #     url: "https://example.com",
  #     kind: "hyperlink",
  #     label: "Example",
  #     page: 1
  #   )
  #
  class Uri < T::Struct
    extend T::Sig

    const :url, String
    const :kind, String
    const :label, T.nilable(String)
    const :page, T.nilable(Integer)
  end

  # Content layer classification for document nodes.
  module ContentLayer
    BODY = 'body'
    HEADER = 'header'
    FOOTER = 'footer'
    FOOTNOTE = 'footnote'

    def self.values
      [BODY, HEADER, FOOTER, FOOTNOTE]
    end
  end

  # Algorithm used for keyword extraction.
  module KeywordAlgorithm
    YAKE = 'yake'
    RAKE = 'rake'

    def self.values
      [YAKE, RAKE]
    end
  end

  # OCR element granularity level.
  module OcrElementLevel
    WORD = 'word'
    LINE = 'line'
    BLOCK = 'block'
    PAGE = 'page'

    def self.values
      [WORD, LINE, BLOCK, PAGE]
    end
  end

  # Output format for extraction results.
  module OutputFormat
    PLAIN = 'plain'
    MARKDOWN = 'markdown'
    DJOT = 'djot'
    HTML = 'html'
    STRUCTURED = 'structured'

    def self.values
      [PLAIN, MARKDOWN, DJOT, HTML, STRUCTURED]
    end
  end

  # Page unit type classification.
  module PageUnitType
    PAGE = 'page'
    SLIDE = 'slide'
    SHEET = 'sheet'

    def self.values
      [PAGE, SLIDE, SHEET]
    end
  end

  # PDF annotation type classification.
  module PdfAnnotationType
    TEXT = 'text'
    HIGHLIGHT = 'highlight'
    LINK = 'link'
    STAMP = 'stamp'
    UNDERLINE = 'underline'
    STRIKE_OUT = 'strike_out'
    OTHER = 'other'

    def self.values
      [TEXT, HIGHLIGHT, LINK, STAMP, UNDERLINE, STRIKE_OUT, OTHER]
    end
  end

  # Relationship kind between document elements.
  module RelationshipKind
    FOOTNOTE_REFERENCE = 'footnote_reference'
    CITATION_REFERENCE = 'citation_reference'
    INTERNAL_LINK = 'internal_link'
    CAPTION = 'caption'
    LABEL = 'label'
    TOC_ENTRY = 'toc_entry'
    CROSS_REFERENCE = 'cross_reference'

    def self.values
      [FOOTNOTE_REFERENCE, CITATION_REFERENCE, INTERNAL_LINK, CAPTION, LABEL, TOC_ENTRY, CROSS_REFERENCE]
    end
  end

  # Result format classification.
  module ResultFormat
    UNIFIED = 'unified'
    ELEMENT_BASED = 'element_based'

    def self.values
      [UNIFIED, ELEMENT_BASED]
    end
  end

  # URI kind classification.
  module UriKind
    HYPERLINK = 'hyperlink'
    IMAGE = 'image'
    ANCHOR = 'anchor'
    CITATION = 'citation'
    REFERENCE = 'reference'
    EMAIL = 'email'

    def self.values
      [HYPERLINK, IMAGE, ANCHOR, CITATION, REFERENCE, EMAIL]
    end
  end
end
