# frozen_string_literal: true

require 'sorbet-runtime'

module Kreuzberg
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
end
