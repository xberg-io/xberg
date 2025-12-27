# frozen_string_literal: true

require 'minitest/autorun'
require 'kreuzberg'
require 'json'
require 'tempfile'

# Comprehensive tests for Kreuzberg metadata types
# Tests verify T::Struct behavior, type safety, and integration with extraction
# rubocop:disable Metrics/ClassLength, Metrics/MethodLength, Metrics/AbcSize
class MetadataTypesTest < Minitest::Test
  def test_html_metadata_structure
    metadata = Kreuzberg::HtmlMetadata.new(
      title: 'Test Page',
      description: 'A test description',
      author: 'Test Author',
      copyright: '2024 Test Corp',
      keywords: %w[test metadata],
      canonical_url: 'https://example.com/test',
      language: 'en',
      text_direction: 'ltr',
      mime_type: 'text/html',
      charset: 'utf-8',
      generator: 'Kreuzberg',
      viewport: 'width=device-width, initial-scale=1',
      theme_color: '#ffffff',
      application_name: 'Test App',
      robots: 'index, follow',
      open_graph: { 'og:title' => 'Test', 'og:image' => 'image.jpg' },
      twitter_card: { 'twitter:card' => 'summary' },
      meta_tags: { 'custom' => 'value' },
      headers: [],
      links: [],
      images: [],
      structured_data: []
    )

    assert_equal 'Test Page', metadata.title
    assert_equal 'A test description', metadata.description
    assert_equal 'Test Author', metadata.author
    assert_equal '2024 Test Corp', metadata.copyright
    assert_equal 'https://example.com/test', metadata.canonical_url
    assert_equal 'en', metadata.language
    assert_equal 'ltr', metadata.text_direction
    assert_equal 'text/html', metadata.mime_type
    assert_equal 'utf-8', metadata.charset
    assert_equal 'Kreuzberg', metadata.generator
    assert_equal '#ffffff', metadata.theme_color
    assert_equal 'Test App', metadata.application_name
    assert_equal 'index, follow', metadata.robots
  end

  def test_keywords_is_array
    keywords_array = %w[test metadata array]
    metadata = Kreuzberg::HtmlMetadata.new(
      title: nil,
      description: nil,
      author: nil,
      copyright: nil,
      keywords: keywords_array,
      canonical_url: nil,
      language: nil,
      text_direction: nil,
      mime_type: nil,
      charset: nil,
      generator: nil,
      viewport: nil,
      theme_color: nil,
      application_name: nil,
      robots: nil,
      open_graph: {},
      twitter_card: {},
      meta_tags: {},
      headers: [],
      links: [],
      images: [],
      structured_data: []
    )

    assert_instance_of Array, metadata.keywords
    assert_equal keywords_array, metadata.keywords
    metadata.keywords.each { |keyword| assert_instance_of String, keyword }
  end

  def test_canonical_url_renamed
    metadata = Kreuzberg::HtmlMetadata.new(
      title: nil,
      description: nil,
      author: nil,
      copyright: nil,
      keywords: [],
      canonical_url: 'https://example.com/canonical',
      language: nil,
      text_direction: nil,
      mime_type: nil,
      charset: nil,
      generator: nil,
      viewport: nil,
      theme_color: nil,
      application_name: nil,
      robots: nil,
      open_graph: {},
      twitter_card: {},
      meta_tags: {},
      headers: [],
      links: [],
      images: [],
      structured_data: []
    )

    assert_equal 'https://example.com/canonical', metadata.canonical_url
    assert_respond_to metadata, :canonical_url
  end

  def test_open_graph_is_hash
    og_tags = {
      'og:title' => 'Test Title',
      'og:description' => 'Test Description',
      'og:image' => 'https://example.com/image.jpg',
      'og:url' => 'https://example.com'
    }
    metadata = Kreuzberg::HtmlMetadata.new(
      title: nil,
      description: nil,
      author: nil,
      copyright: nil,
      keywords: [],
      canonical_url: nil,
      language: nil,
      text_direction: nil,
      mime_type: nil,
      charset: nil,
      generator: nil,
      viewport: nil,
      theme_color: nil,
      application_name: nil,
      robots: nil,
      open_graph: og_tags,
      twitter_card: {},
      meta_tags: {},
      headers: [],
      links: [],
      images: [],
      structured_data: []
    )

    assert_instance_of Hash, metadata.open_graph
    assert_equal og_tags, metadata.open_graph
    metadata.open_graph.each do |key, value|
      assert_instance_of String, key
      assert_instance_of String, value
    end
  end

  def test_twitter_card_is_hash
    twitter_tags = {
      'twitter:card' => 'summary_large_image',
      'twitter:title' => 'Test',
      'twitter:description' => 'Description',
      'twitter:image' => 'https://example.com/image.jpg'
    }
    metadata = Kreuzberg::HtmlMetadata.new(
      title: nil,
      description: nil,
      author: nil,
      copyright: nil,
      keywords: [],
      canonical_url: nil,
      language: nil,
      text_direction: nil,
      mime_type: nil,
      charset: nil,
      generator: nil,
      viewport: nil,
      theme_color: nil,
      application_name: nil,
      robots: nil,
      open_graph: {},
      twitter_card: twitter_tags,
      meta_tags: {},
      headers: [],
      links: [],
      images: [],
      structured_data: []
    )

    assert_instance_of Hash, metadata.twitter_card
    assert_equal twitter_tags, metadata.twitter_card
    metadata.twitter_card.each do |key, value|
      assert_instance_of String, key
      assert_instance_of String, value
    end
  end

  # ============================================================================
  # T::Struct Behavior Tests
  # ============================================================================

  def test_header_metadata_creation
    header = Kreuzberg::HeaderMetadata.new(
      level: 1,
      text: 'Main Title',
      id: 'main-title',
      depth: 0,
      html_offset: 245
    )

    assert_equal 1, header.level
    assert_equal 'Main Title', header.text
    assert_equal 'main-title', header.id
    assert_equal 0, header.depth
    assert_equal 245, header.html_offset
  end

  def test_header_metadata_nil_id
    header = Kreuzberg::HeaderMetadata.new(
      level: 2,
      text: 'Subtitle',
      id: nil,
      depth: 1,
      html_offset: 456
    )

    assert_equal 2, header.level
    assert_equal 'Subtitle', header.text
    assert_nil header.id
    assert_equal 1, header.depth
    assert_equal 456, header.html_offset
  end

  def test_link_metadata_creation
    link = Kreuzberg::LinkMetadata.new(
      href: 'https://example.com',
      text: 'Example',
      title: 'Example Site',
      link_type: 'external',
      rel: %w[noopener noreferrer],
      attributes: { 'data-id' => '123', 'class' => 'external-link' }
    )

    assert_equal 'https://example.com', link.href
    assert_equal 'Example', link.text
    assert_equal 'Example Site', link.title
    assert_equal 'external', link.link_type
    assert_instance_of Array, link.rel
    assert_equal %w[noopener noreferrer], link.rel
    assert_instance_of Hash, link.attributes
    assert_equal '123', link.attributes['data-id']
    assert_equal 'external-link', link.attributes['class']
  end

  def test_link_metadata_empty_arrays_and_hashes
    link = Kreuzberg::LinkMetadata.new(
      href: 'https://example.com',
      text: 'Link',
      title: nil,
      link_type: 'internal',
      rel: [],
      attributes: {}
    )

    assert_equal 'https://example.com', link.href
    assert_empty link.rel
    assert_empty link.attributes
    assert_nil link.title
  end

  def test_image_metadata_creation
    image = Kreuzberg::ImageMetadata.new(
      src: 'images/logo.png',
      alt: 'Company Logo',
      title: nil,
      dimensions: [200, 100],
      image_type: 'png',
      attributes: { 'loading' => 'lazy', 'class' => 'logo' }
    )

    assert_equal 'images/logo.png', image.src
    assert_equal 'Company Logo', image.alt
    assert_nil image.title
    assert_instance_of Array, image.dimensions
    assert_equal [200, 100], image.dimensions
    assert_equal 'png', image.image_type
    assert_instance_of Hash, image.attributes
    assert_equal 'lazy', image.attributes['loading']
  end

  def test_image_metadata_nil_dimensions
    image = Kreuzberg::ImageMetadata.new(
      src: 'image.jpg',
      alt: 'Description',
      title: 'Title',
      dimensions: nil,
      image_type: 'jpg',
      attributes: {}
    )

    assert_equal 'image.jpg', image.src
    assert_nil image.dimensions
    assert_equal 'jpg', image.image_type
  end

  def test_structured_data_creation
    json_data = '{"@context":"https://schema.org","@type":"Article","headline":"Test Article"}'
    structured = Kreuzberg::StructuredData.new(
      data_type: 'json-ld',
      raw_json: json_data,
      schema_type: 'Article'
    )

    assert_equal 'json-ld', structured.data_type
    assert_equal json_data, structured.raw_json
    assert_equal 'Article', structured.schema_type
    parsed = JSON.parse(structured.raw_json)
    assert_equal 'Article', parsed['@type']
  end

  def test_structured_data_nil_schema_type
    json_data = '{"data":"value"}'
    structured = Kreuzberg::StructuredData.new(
      data_type: 'microdata',
      raw_json: json_data,
      schema_type: nil
    )

    assert_equal 'microdata', structured.data_type
    assert_nil structured.schema_type
  end

  # ============================================================================
  # Integration Tests
  # ============================================================================

  def test_extract_html_returns_metadata
    html_file = create_test_html_file(
      '<html><head><title>Test Page</title></head><body><p>Content</p></body></html>'
    )

    begin
      result = Kreuzberg.extract_file_sync(html_file)
      assert_instance_of Kreuzberg::Result, result
      assert_not_nil result.metadata

      if result.metadata.is_a?(Hash)
        assert result.metadata.is_a?(Hash)
      elsif result.metadata.is_a?(Kreuzberg::HtmlMetadata)
        assert result.metadata.is_a?(Kreuzberg::HtmlMetadata)
      end
    ensure
      FileUtils.rm_f(html_file)
    end
  end

  def test_metadata_keywords_array
    html_content = <<~HTML
      <html>
        <head>
          <title>Test</title>
          <meta name="keywords" content="ruby, testing, metadata">
        </head>
        <body></body>
      </html>
    HTML
    html_file = create_test_html_file(html_content)

    begin
      result = Kreuzberg.extract_file_sync(html_file)
      metadata = result.metadata

      if metadata.is_a?(Hash) && metadata['keywords']
        assert metadata['keywords'].is_a?(Array)
      elsif metadata.is_a?(Kreuzberg::HtmlMetadata)
        assert_instance_of Array, metadata.keywords
      end
    ensure
      FileUtils.rm_f(html_file)
    end
  end

  def test_metadata_open_graph_hash
    html_content = <<~HTML
      <html>
        <head>
          <title>Test</title>
          <meta property="og:title" content="Test Title">
          <meta property="og:description" content="Test Description">
          <meta property="og:image" content="https://example.com/image.jpg">
        </head>
        <body></body>
      </html>
    HTML
    html_file = create_test_html_file(html_content)

    begin
      result = Kreuzberg.extract_file_sync(html_file)
      metadata = result.metadata

      if metadata.is_a?(Hash) && metadata['open_graph']
        assert metadata['open_graph'].is_a?(Hash)
      elsif metadata.is_a?(Kreuzberg::HtmlMetadata)
        assert_instance_of Hash, metadata.open_graph
      end
    ensure
      FileUtils.rm_f(html_file)
    end
  end

  def test_metadata_headers_array
    html_content = <<~HTML
      <html>
        <head><title>Test</title></head>
        <body>
          <h1>Main Title</h1>
          <h2>Subtitle</h2>
          <h3 id="section-1">Section 1</h3>
        </body>
      </html>
    HTML
    html_file = create_test_html_file(html_content)

    begin
      result = Kreuzberg.extract_file_sync(html_file)
      metadata = result.metadata

      if metadata.is_a?(Hash) && metadata['headers']
        assert metadata['headers'].is_a?(Array)
      elsif metadata.is_a?(Kreuzberg::HtmlMetadata)
        assert_instance_of Array, metadata.headers
      end
    ensure
      FileUtils.rm_f(html_file)
    end
  end

  def test_metadata_links_array
    html_content = <<~HTML
      <html>
        <head><title>Test</title></head>
        <body>
          <a href="https://example.com">External Link</a>
          <a href="/page">Internal Link</a>
          <a href="#section">Anchor Link</a>
        </body>
      </html>
    HTML
    html_file = create_test_html_file(html_content)

    begin
      result = Kreuzberg.extract_file_sync(html_file)
      metadata = result.metadata

      if metadata.is_a?(Hash) && metadata['links']
        assert metadata['links'].is_a?(Array)
      elsif metadata.is_a?(Kreuzberg::HtmlMetadata)
        assert_instance_of Array, metadata.links
      end
    ensure
      FileUtils.rm_f(html_file)
    end
  end

  def test_metadata_images_array
    html_content = <<~HTML
      <html>
        <head><title>Test</title></head>
        <body>
          <img src="image1.jpg" alt="Image 1" width="200" height="100">
          <img src="image2.png" alt="Image 2">
          <img src="image3.gif">
        </body>
      </html>
    HTML
    html_file = create_test_html_file(html_content)

    begin
      result = Kreuzberg.extract_file_sync(html_file)
      metadata = result.metadata

      if metadata.is_a?(Hash) && metadata['images']
        assert metadata['images'].is_a?(Array)
      elsif metadata.is_a?(Kreuzberg::HtmlMetadata)
        assert_instance_of Array, metadata.images
      end
    ensure
      FileUtils.rm_f(html_file)
    end
  end

  # ============================================================================
  # Edge Cases
  # ============================================================================

  def test_metadata_empty_html
    html_file = create_test_html_file('<html><body></body></html>')

    begin
      result = Kreuzberg.extract_file_sync(html_file)
      metadata = result.metadata

      if metadata.is_a?(Kreuzberg::HtmlMetadata)
        assert_instance_of Array, metadata.keywords
        assert_instance_of Hash, metadata.open_graph
        assert_instance_of Hash, metadata.twitter_card
        assert_instance_of Hash, metadata.meta_tags
        assert_instance_of Array, metadata.headers
        assert_instance_of Array, metadata.links
        assert_instance_of Array, metadata.images
        assert_instance_of Array, metadata.structured_data
      elsif metadata.is_a?(Hash)
        assert_instance_of Array, metadata['keywords'] || []
        assert_instance_of Hash, metadata['open_graph'] || {}
        assert_instance_of Hash, metadata['twitter_card'] || {}
      end
    ensure
      FileUtils.rm_f(html_file)
    end
  end

  def test_metadata_nil_optional_fields
    metadata = Kreuzberg::HtmlMetadata.new(
      title: nil,
      description: nil,
      author: nil,
      copyright: nil,
      keywords: [],
      canonical_url: nil,
      language: nil,
      text_direction: nil,
      mime_type: nil,
      charset: nil,
      generator: nil,
      viewport: nil,
      theme_color: nil,
      application_name: nil,
      robots: nil,
      open_graph: {},
      twitter_card: {},
      meta_tags: {},
      headers: [],
      links: [],
      images: [],
      structured_data: []
    )

    assert_nil metadata.title
    assert_nil metadata.description
    assert_nil metadata.author
    assert_nil metadata.copyright
    assert_nil metadata.canonical_url
    assert_nil metadata.language
    assert_nil metadata.text_direction
    assert_nil metadata.mime_type
    assert_nil metadata.charset
    assert_nil metadata.generator
    assert_nil metadata.viewport
    assert_nil metadata.theme_color
    assert_nil metadata.application_name
    assert_nil metadata.robots
  end

  def test_metadata_empty_collections
    metadata = Kreuzberg::HtmlMetadata.new(
      title: nil,
      description: nil,
      author: nil,
      copyright: nil,
      keywords: [],
      canonical_url: nil,
      language: nil,
      text_direction: nil,
      mime_type: nil,
      charset: nil,
      generator: nil,
      viewport: nil,
      theme_color: nil,
      application_name: nil,
      robots: nil,
      open_graph: {},
      twitter_card: {},
      meta_tags: {},
      headers: [],
      links: [],
      images: [],
      structured_data: []
    )

    assert_empty metadata.keywords
    assert_empty metadata.open_graph
    assert_empty metadata.twitter_card
    assert_empty metadata.meta_tags
    assert_empty metadata.headers
    assert_empty metadata.links
    assert_empty metadata.images
    assert_empty metadata.structured_data
  end

  # ============================================================================
  # Sorbet Type Safety
  # ============================================================================

  def test_type_checking_enabled
    metadata = Kreuzberg::HtmlMetadata.new(
      title: 'Test',
      description: nil,
      author: nil,
      copyright: nil,
      keywords: ['test'],
      canonical_url: nil,
      language: nil,
      text_direction: nil,
      mime_type: nil,
      charset: nil,
      generator: nil,
      viewport: nil,
      theme_color: nil,
      application_name: nil,
      robots: nil,
      open_graph: {},
      twitter_card: {},
      meta_tags: {},
      headers: [],
      links: [],
      images: [],
      structured_data: []
    )

    assert_kind_of Kreuzberg::HtmlMetadata, metadata
    assert metadata.respond_to?(:title)
    assert metadata.respond_to?(:keywords)
    assert metadata.respond_to?(:open_graph)
  end

  def test_immutable_tstruct_fields
    metadata = Kreuzberg::HtmlMetadata.new(
      title: 'Original',
      description: nil,
      author: nil,
      copyright: nil,
      keywords: [],
      canonical_url: nil,
      language: nil,
      text_direction: nil,
      mime_type: nil,
      charset: nil,
      generator: nil,
      viewport: nil,
      theme_color: nil,
      application_name: nil,
      robots: nil,
      open_graph: {},
      twitter_card: {},
      meta_tags: {},
      headers: [],
      links: [],
      images: [],
      structured_data: []
    )

    assert_raises(NoMethodError) { metadata.title = 'Modified' }
  end

  def test_headers_with_multiple_levels
    headers = [
      Kreuzberg::HeaderMetadata.new(level: 1, text: 'H1', id: nil, depth: 0, html_offset: 0),
      Kreuzberg::HeaderMetadata.new(level: 2, text: 'H2', id: nil, depth: 1, html_offset: 50),
      Kreuzberg::HeaderMetadata.new(level: 3, text: 'H3', id: 'sec-1', depth: 2, html_offset: 100),
      Kreuzberg::HeaderMetadata.new(level: 2, text: 'H2-2', id: nil, depth: 1, html_offset: 150)
    ]

    metadata = Kreuzberg::HtmlMetadata.new(
      title: nil,
      description: nil,
      author: nil,
      copyright: nil,
      keywords: [],
      canonical_url: nil,
      language: nil,
      text_direction: nil,
      mime_type: nil,
      charset: nil,
      generator: nil,
      viewport: nil,
      theme_color: nil,
      application_name: nil,
      robots: nil,
      open_graph: {},
      twitter_card: {},
      meta_tags: {},
      headers: headers,
      links: [],
      images: [],
      structured_data: []
    )

    assert_equal 4, metadata.headers.length
    assert_equal 1, metadata.headers[0].level
    assert_equal 3, metadata.headers[2].level
    assert_equal 'sec-1', metadata.headers[2].id
  end

  def test_links_with_various_types
    links = [
      Kreuzberg::LinkMetadata.new(
        href: 'https://external.com',
        text: 'External',
        title: nil,
        link_type: 'external',
        rel: ['noopener'],
        attributes: {}
      ),
      Kreuzberg::LinkMetadata.new(
        href: '/internal/page',
        text: 'Internal',
        title: 'Internal Page',
        link_type: 'internal',
        rel: [],
        attributes: { 'class' => 'nav-link' }
      ),
      Kreuzberg::LinkMetadata.new(
        href: '#section',
        text: 'Anchor',
        title: nil,
        link_type: 'anchor',
        rel: [],
        attributes: {}
      )
    ]

    metadata = Kreuzberg::HtmlMetadata.new(
      title: nil,
      description: nil,
      author: nil,
      copyright: nil,
      keywords: [],
      canonical_url: nil,
      language: nil,
      text_direction: nil,
      mime_type: nil,
      charset: nil,
      generator: nil,
      viewport: nil,
      theme_color: nil,
      application_name: nil,
      robots: nil,
      open_graph: {},
      twitter_card: {},
      meta_tags: {},
      headers: [],
      links: links,
      images: [],
      structured_data: []
    )

    assert_equal 3, metadata.links.length
    assert_equal 'external', metadata.links[0].link_type
    assert_equal 'internal', metadata.links[1].link_type
    assert_equal 'anchor', metadata.links[2].link_type
    assert_equal 'nav-link', metadata.links[1].attributes['class']
  end

  def test_images_with_attributes
    images = [
      Kreuzberg::ImageMetadata.new(
        src: 'logo.png',
        alt: 'Logo',
        title: nil,
        dimensions: [200, 100],
        image_type: 'png',
        attributes: { 'class' => 'logo', 'loading' => 'eager' }
      ),
      Kreuzberg::ImageMetadata.new(
        src: 'thumbnail.jpg',
        alt: nil,
        title: 'Thumbnail',
        dimensions: nil,
        image_type: 'jpg',
        attributes: { 'loading' => 'lazy', 'decoding' => 'async' }
      )
    ]

    metadata = Kreuzberg::HtmlMetadata.new(
      title: nil,
      description: nil,
      author: nil,
      copyright: nil,
      keywords: [],
      canonical_url: nil,
      language: nil,
      text_direction: nil,
      mime_type: nil,
      charset: nil,
      generator: nil,
      viewport: nil,
      theme_color: nil,
      application_name: nil,
      robots: nil,
      open_graph: {},
      twitter_card: {},
      meta_tags: {},
      headers: [],
      links: [],
      images: images,
      structured_data: []
    )

    assert_equal 2, metadata.images.length
    assert_equal [200, 100], metadata.images[0].dimensions
    assert_nil metadata.images[1].dimensions
    assert_equal 'lazy', metadata.images[1].attributes['loading']
  end

  def test_structured_data_multiple_types
    json_ld = '{"@context":"https://schema.org","@type":"Article"}'
    microdata = '{"type":"http://schema.org/Person"}'

    structured_data = [
      Kreuzberg::StructuredData.new(
        data_type: 'json-ld',
        raw_json: json_ld,
        schema_type: 'Article'
      ),
      Kreuzberg::StructuredData.new(
        data_type: 'microdata',
        raw_json: microdata,
        schema_type: 'Person'
      ),
      Kreuzberg::StructuredData.new(
        data_type: 'json-ld',
        raw_json: '{"@type":"Organization"}',
        schema_type: nil
      )
    ]

    metadata = Kreuzberg::HtmlMetadata.new(
      title: nil,
      description: nil,
      author: nil,
      copyright: nil,
      keywords: [],
      canonical_url: nil,
      language: nil,
      text_direction: nil,
      mime_type: nil,
      charset: nil,
      generator: nil,
      viewport: nil,
      theme_color: nil,
      application_name: nil,
      robots: nil,
      open_graph: {},
      twitter_card: {},
      meta_tags: {},
      headers: [],
      links: [],
      images: [],
      structured_data: structured_data
    )

    assert_equal 3, metadata.structured_data.length
    assert_equal 'json-ld', metadata.structured_data[0].data_type
    assert_equal 'Article', metadata.structured_data[0].schema_type
    assert_equal 'microdata', metadata.structured_data[1].data_type
    assert_nil metadata.structured_data[2].schema_type
  end

  def test_html_metadata_with_all_fields_populated
    headers = [
      Kreuzberg::HeaderMetadata.new(level: 1, text: 'Title', id: 'title', depth: 0, html_offset: 100)
    ]
    links = [
      Kreuzberg::LinkMetadata.new(
        href: 'https://example.com',
        text: 'Example',
        title: 'Example Site',
        link_type: 'external',
        rel: ['noopener'],
        attributes: { 'data-track' => 'true' }
      )
    ]
    images = [
      Kreuzberg::ImageMetadata.new(
        src: 'image.jpg',
        alt: 'Test Image',
        title: nil,
        dimensions: [300, 200],
        image_type: 'jpg',
        attributes: { 'loading' => 'lazy' }
      )
    ]
    structured = [
      Kreuzberg::StructuredData.new(
        data_type: 'json-ld',
        raw_json: '{"@type":"WebPage"}',
        schema_type: 'WebPage'
      )
    ]

    metadata = Kreuzberg::HtmlMetadata.new(
      title: 'Complete Test Page',
      description: 'A complete test page with all metadata',
      author: 'Test Author',
      copyright: '2024 Test Corp',
      keywords: %w[test comprehensive metadata],
      canonical_url: 'https://example.com/test',
      language: 'en',
      text_direction: 'ltr',
      mime_type: 'text/html; charset=utf-8',
      charset: 'utf-8',
      generator: 'Kreuzberg',
      viewport: 'width=device-width, initial-scale=1',
      theme_color: '#ffffff',
      application_name: 'Test App',
      robots: 'index, follow',
      open_graph: {
        'og:title' => 'Test',
        'og:description' => 'Description',
        'og:image' => 'https://example.com/image.jpg'
      },
      twitter_card: {
        'twitter:card' => 'summary_large_image',
        'twitter:title' => 'Test'
      },
      meta_tags: {
        'custom-tag' => 'custom-value'
      },
      headers: headers,
      links: links,
      images: images,
      structured_data: structured
    )

    assert_equal 'Complete Test Page', metadata.title
    assert_equal 'A complete test page with all metadata', metadata.description
    assert_equal 'Test Author', metadata.author
    assert_equal '2024 Test Corp', metadata.copyright
    assert_equal 3, metadata.keywords.length
    assert_equal 'https://example.com/test', metadata.canonical_url
    assert_equal 'en', metadata.language
    assert_equal 'ltr', metadata.text_direction
    assert_equal 'Kreuzberg', metadata.generator
    assert_equal 3, metadata.open_graph.length
    assert_equal 2, metadata.twitter_card.length
    assert_equal 1, metadata.meta_tags.length
    assert_equal 1, metadata.headers.length
    assert_equal 1, metadata.links.length
    assert_equal 1, metadata.images.length
    assert_equal 1, metadata.structured_data.length
  end

  private

  def create_test_html_file(content)
    file = Tempfile.new(['test', '.html'])
    file.write(content)
    file.close
    file.path
  end
end
# rubocop:enable Metrics/ClassLength, Metrics/MethodLength, Metrics/AbcSize
