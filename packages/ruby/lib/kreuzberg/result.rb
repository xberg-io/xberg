# frozen_string_literal: true

begin
  require 'json'
rescue LoadError
  require 'json/pure'
end

require_relative 'document_structure'

module Kreuzberg
  # @example
  # rubocop:disable Metrics/ClassLength
  class Result
    attr_reader :content, :mime_type, :metadata, :metadata_json, :tables,
                :detected_languages, :chunks, :images, :pages, :elements, :ocr_elements, :djot_content,
                :document, :extracted_keywords, :quality_score, :processing_warnings, :annotations,
                :uris, :children, :structured_output

    # @!attribute [r] cells
    #   @return [Array<Array<String>>] Table cells (2D array)
    # @!attribute [r] markdown
    #   @return [String] Markdown representation
    # @!attribute [r] page_number
    #   @return [Integer] Page number where table was found
    # @!attribute [r] bounding_box
    #   @return [BoundingBox, nil] Bounding box of the table on the page
    Table = Struct.new(:cells, :markdown, :page_number, :bounding_box) do
      def to_h
        { cells: cells, markdown: markdown, page_number: page_number, bounding_box: bounding_box&.to_h }
      end
    end

    # @!attribute [r] content
    #   @return [String] Chunk content
    # @!attribute [r] byte_start
    #   @return [Integer] Starting byte offset (UTF-8)
    # @!attribute [r] byte_end
    #   @return [Integer] Ending byte offset (UTF-8)
    # @!attribute [r] token_count
    #   @return [Integer, nil] Approximate token count (may be nil)
    # @!attribute [r] first_page
    #   @return [Integer, nil] First page number (1-indexed)
    # @!attribute [r] last_page
    #   @return [Integer, nil] Last page number (1-indexed)
    Chunk = Struct.new(
      :content,
      :byte_start,
      :byte_end,
      :token_count,
      :chunk_index,
      :total_chunks,
      :first_page,
      :last_page,
      :chunk_type,
      :embedding
    ) do
      def to_h
        {
          content: content,
          byte_start: byte_start,
          byte_end: byte_end,
          token_count: token_count,
          chunk_index: chunk_index,
          total_chunks: total_chunks,
          first_page: first_page,
          last_page: last_page,
          chunk_type: chunk_type,
          embedding: embedding
        }
      end
    end

    Image = Struct.new(
      :data,
      :format,
      :image_index,
      :page_number,
      :width,
      :height,
      :colorspace,
      :bits_per_component,
      :is_mask,
      :description,
      :bounding_box,
      :ocr_result
    ) do
      def to_h
        {
          data: data,
          format: format,
          image_index: image_index,
          page_number: page_number,
          width: width,
          height: height,
          colorspace: colorspace,
          bits_per_component: bits_per_component,
          is_mask: is_mask,
          description: description,
          bounding_box: bounding_box&.to_h,
          ocr_result: ocr_result&.to_h
        }
      end
    end

    # @!attribute [r] page_number
    #   @return [Integer] Page number (1-indexed)
    # @!attribute [r] content
    #   @return [String] Text content for this page
    # @!attribute [r] tables
    #   @return [Array<Table>] Tables on this page
    # @!attribute [r] images
    #   @return [Array<Image>] Images on this page
    # @!attribute [r] text
    #   @return [String] The text content of this block
    # @!attribute [r] font_size
    #   @return [Float] The font size of the text
    # @!attribute [r] level
    #   @return [String] The hierarchy level (h1-h6 or body)
    # @!attribute [r] bbox
    #   @return [Array<Float>, nil] Bounding box (left, top, right, bottom)
    HierarchicalBlock = Struct.new(:text, :font_size, :level, :bbox) do
      def to_h
        { text: text, font_size: font_size, level: level, bbox: bbox }
      end
    end

    # @!attribute [r] block_count
    #   @return [Integer] Number of hierarchy blocks
    # @!attribute [r] blocks
    #   @return [Array<HierarchicalBlock>] Hierarchical blocks
    PageHierarchy = Struct.new(:block_count, :blocks) do
      def to_h
        { block_count: block_count, blocks: blocks.map(&:to_h) }
      end
    end

    # @!attribute [r] page_number
    #   @return [Integer] Page number (1-indexed)
    # @!attribute [r] content
    #   @return [String] Text content for this page
    # @!attribute [r] tables
    #   @return [Array<Table>] Tables on this page
    # @!attribute [r] images
    #   @return [Array<Image>] Images on this page
    # @!attribute [r] hierarchy
    #   @return [PageHierarchy, nil] Hierarchy information for the page
    PageContent = Struct.new(:page_number, :content, :tables, :images, :hierarchy, :is_blank, :layout_regions) do
      def to_h
        {
          page_number: page_number,
          content: content,
          tables: tables.map(&:to_h),
          images: images.map(&:to_h),
          hierarchy: hierarchy&.to_h,
          is_blank: is_blank,
          layout_regions: layout_regions&.map(&:to_h)
        }
      end
    end

    # @!attribute [r] class_name
    #   @return [String] Layout class name (e.g. "picture", "table", "text")
    # @!attribute [r] confidence
    #   @return [Float] Detection confidence score (0.0 to 1.0)
    # @!attribute [r] bounding_box
    #   @return [ElementBoundingBox] Bounding box in document coordinate space
    # @!attribute [r] area_fraction
    #   @return [Float] Fraction of page area covered (0.0 to 1.0)
    LayoutRegion = Struct.new(:class_name, :confidence, :bounding_box, :area_fraction) do
      def to_h
        {
          class: class_name,
          confidence: confidence,
          bounding_box: bounding_box&.to_h,
          area_fraction: area_fraction
        }
      end
    end

    # @!attribute [r] x0
    #   @return [Float] Left x-coordinate
    # @!attribute [r] y0
    #   @return [Float] Bottom y-coordinate
    # @!attribute [r] x1
    #   @return [Float] Right x-coordinate
    # @!attribute [r] y1
    #   @return [Float] Top y-coordinate
    ElementBoundingBox = Struct.new(:x0, :y0, :x1, :y1) do
      def to_h
        { x0: x0, y0: y0, x1: x1, y1: y1 }
      end
    end

    # @!attribute [r] page_number
    #   @return [Integer, nil] Page number (1-indexed)
    # @!attribute [r] filename
    #   @return [String, nil] Source filename or document name
    # @!attribute [r] coordinates
    #   @return [ElementBoundingBox, nil] Bounding box coordinates if available
    # @!attribute [r] element_index
    #   @return [Integer, nil] Position index in the element sequence
    # @!attribute [r] additional
    #   @return [Hash<String, String>] Additional custom metadata
    ElementMetadataStruct = Struct.new(
      :page_number,
      :filename,
      :coordinates,
      :element_index,
      :additional
    ) do
      def to_h
        {
          page_number: page_number,
          filename: filename,
          coordinates: coordinates&.to_h,
          element_index: element_index,
          additional: additional
        }
      end
    end

    # @!attribute [r] element_id
    #   @return [String] Unique element identifier
    # @!attribute [r] element_type
    #   @return [String] Semantic type of the element
    # @!attribute [r] text
    #   @return [String] Text content of the element
    # @!attribute [r] metadata
    #   @return [ElementMetadataStruct] Metadata about the element
    ElementStruct = Struct.new(:element_id, :element_type, :text, :metadata) do
      def to_h
        {
          element_id: element_id,
          element_type: element_type,
          text: text,
          metadata: metadata&.to_h
        }
      end
    end

    # OCR bounding geometry with type and coordinates
    class OcrBoundingGeometry
      attr_reader :type, :left, :top, :width, :height, :points

      def initialize(type:, left: nil, top: nil, width: nil, height: nil, points: nil)
        @type = type.to_s
        @left = left&.to_f
        @top = top&.to_f
        @width = width&.to_f
        @height = height&.to_f
        @points = points
      end

      def to_h
        {
          type: @type,
          left: @left,
          top: @top,
          width: @width,
          height: @height,
          points: @points
        }.compact
      end
    end

    # OCR confidence scores for detection and recognition
    class OcrConfidence
      attr_reader :detection, :recognition

      def initialize(detection: nil, recognition: nil)
        @detection = detection&.to_f
        @recognition = recognition&.to_f
      end

      def to_h
        {
          detection: @detection,
          recognition: @recognition
        }.compact
      end
    end

    # OCR rotation information
    class OcrRotation
      attr_reader :angle_degrees, :confidence

      def initialize(angle_degrees: nil, confidence: nil)
        @angle_degrees = angle_degrees&.to_f
        @confidence = confidence&.to_f
      end

      def to_h
        {
          angle_degrees: @angle_degrees,
          confidence: @confidence
        }.compact
      end
    end

    # OCR text element with geometry and metadata
    class OcrElement
      attr_reader :text, :geometry, :confidence, :level, :rotation,
                  :page_number, :parent_id, :backend_metadata

      def initialize(
        text:,
        geometry: nil,
        confidence: nil,
        level: nil,
        rotation: nil,
        page_number: nil,
        parent_id: nil,
        backend_metadata: nil
      )
        @text = text.to_s
        @geometry = geometry
        @confidence = confidence
        @level = level&.to_s
        @rotation = rotation
        @page_number = page_number&.to_i
        @parent_id = parent_id&.to_s
        @backend_metadata = backend_metadata
      end

      def to_h
        {
          text: @text,
          geometry: @geometry&.to_h,
          confidence: @confidence&.to_h,
          level: @level,
          rotation: @rotation&.to_h,
          page_number: @page_number,
          parent_id: @parent_id,
          backend_metadata: @backend_metadata
        }.compact
      end
    end

    # Initialize from native hash result
    #
    # @param hash [Hash] Hash returned from native extension
    #
    # rubocop:disable Metrics/AbcSize, Metrics/MethodLength
    def initialize(hash)
      @content = get_value(hash, 'content', '')
      @mime_type = get_value(hash, 'mime_type', '')
      @metadata_json = get_value(hash, 'metadata_json', '{}')
      @metadata = parse_metadata(@metadata_json)
      @tables = parse_tables(get_value(hash, 'tables'))
      @detected_languages = parse_detected_languages(get_value(hash, 'detected_languages'))
      @chunks = parse_chunks(get_value(hash, 'chunks'))
      @images = parse_images(get_value(hash, 'images'))
      @pages = parse_pages(get_value(hash, 'pages'))
      @elements = parse_elements(get_value(hash, 'elements'))
      @ocr_elements = parse_ocr_elements(get_value(hash, 'ocr_elements'))
      @djot_content = parse_djot_content(get_value(hash, 'djot_content'))
      @document = parse_document_structure(get_value(hash, 'document'))
      @extracted_keywords = parse_extracted_keywords(get_value(hash, 'extracted_keywords'))
      @quality_score = get_value(hash, 'quality_score')
      @processing_warnings = parse_processing_warnings(get_value(hash, 'processing_warnings'))
      @annotations = parse_annotations(get_value(hash, 'annotations'))
      @uris = parse_uris(get_value(hash, 'uris'))
      @children = parse_children(get_value(hash, 'children'))
      @structured_output = get_value(hash, 'structured_output')
    end
    # rubocop:enable Metrics/AbcSize, Metrics/MethodLength

    # Convert to hash
    #
    # @return [Hash] Hash representation
    #
    # rubocop:disable Metrics/CyclomaticComplexity, Metrics/MethodLength
    def to_h
      {
        content: @content,
        mime_type: @mime_type,
        metadata: @metadata,
        tables: serialize_tables,
        detected_languages: @detected_languages,
        chunks: serialize_chunks,
        images: serialize_images,
        pages: serialize_pages,
        elements: serialize_elements,
        ocr_elements: serialize_ocr_elements,
        djot_content: @djot_content&.to_h,
        document: @document&.to_h,
        extracted_keywords: @extracted_keywords&.map(&:to_h),
        quality_score: @quality_score,
        processing_warnings: @processing_warnings.map(&:to_h),
        annotations: @annotations&.map(&:to_h),
        uris: @uris&.map(&:to_h),
        children: @children&.map(&:to_h),
        structured_output: @structured_output
      }
    end
    # rubocop:enable Metrics/CyclomaticComplexity, Metrics/MethodLength

    # Convert to JSON
    #
    # @return [String] JSON representation
    #
    def to_json(*)
      to_h.to_json(*)
    end

    # Get the total number of pages in the document
    #
    # @return [Integer] Total page count (>= 0), or -1 on error
    #
    # @example
    #   result = Kreuzberg.extract_file_sync("document.pdf")
    #   puts "Document has #{result.page_count} pages"
    #
    def page_count
      if @metadata.is_a?(Hash) && @metadata['pages'].is_a?(Hash)
        @metadata['pages']['total_count'] || 0
      else
        0
      end
    end

    # Get the total number of text chunks
    #
    # Returns 0 if chunking was not performed.
    #
    # @return [Integer] Total chunk count (>= 0), or -1 on error
    #
    # @example
    #   result = Kreuzberg.extract_file_sync("document.pdf")
    #   puts "Document has #{result.chunk_count} chunks"
    #
    def chunk_count
      @chunks&.length || 0
    end

    # Get the primary detected language
    #
    # @return [String, nil] ISO 639 language code (e.g., "en", "de"), or nil if not detected
    #
    # @example
    #   result = Kreuzberg.extract_file_sync("document.pdf")
    #   lang = result.detected_language
    #   puts "Language: #{lang}" if lang
    #
    def detected_language
      return @metadata['language'] if @metadata.is_a?(Hash) && @metadata['language']
      return @detected_languages&.first if @detected_languages&.any?

      nil
    end

    # Get a metadata field by name
    #
    # Supports dot notation for nested fields (e.g., "format.pages").
    #
    # @param name [String, Symbol] Field name
    # @return [Object, nil] Field value, or nil if field doesn't exist
    #
    # @example Get a top-level field
    #   result = Kreuzberg.extract_file_sync("document.pdf")
    #   title = result.metadata_field("title")
    #   puts "Title: #{title}" if title
    #
    # @example Get a nested field
    #   format_info = result.metadata_field("format.pages")
    #
    def metadata_field(name)
      return nil unless @metadata.is_a?(Hash)

      parts = name.to_s.split('.')
      value = @metadata

      parts.each do |part|
        return nil unless value.is_a?(Hash)

        value = value[part]
      end

      value
    end

    private

    def serialize_tables
      @tables.map(&:to_h)
    end

    def serialize_chunks
      @chunks&.map(&:to_h)
    end

    def serialize_images
      @images&.map(&:to_h)
    end

    def serialize_pages
      @pages&.map(&:to_h)
    end

    def serialize_elements
      @elements&.map(&:to_h)
    end

    def serialize_ocr_elements
      @ocr_elements&.map(&:to_h)
    end

    def get_value(hash, key, default = nil)
      hash[key] || hash[key.to_sym] || default
    end

    def parse_metadata(metadata_json)
      JSON.parse(metadata_json)
    rescue JSON::ParserError
      {}
    end

    def parse_tables(tables_data)
      return [] if tables_data.nil? || tables_data.empty?

      tables_data.map do |table_hash|
        bounding_box = parse_bounding_box(table_hash['bounding_box'])
        Table.new(
          cells: table_hash['cells'] || [],
          markdown: table_hash['markdown'] || '',
          page_number: table_hash['page_number'] || 0,
          bounding_box: bounding_box
        )
      end
    end

    def parse_detected_languages(langs_data)
      return nil if langs_data.nil?

      langs_data.is_a?(Array) ? langs_data : []
    end

    def parse_chunks(chunks_data)
      return [] if chunks_data.nil? || chunks_data.empty?

      chunks_data.map do |chunk_hash|
        Chunk.new(
          content: chunk_hash['content'],
          byte_start: chunk_hash['byte_start'],
          byte_end: chunk_hash['byte_end'],
          token_count: chunk_hash['token_count'],
          chunk_index: chunk_hash['chunk_index'],
          total_chunks: chunk_hash['total_chunks'],
          first_page: chunk_hash['first_page'],
          last_page: chunk_hash['last_page'],
          chunk_type: chunk_hash['chunk_type'],
          embedding: chunk_hash['embedding']
        )
      end
    end

    def parse_images(images_data)
      return nil if images_data.nil?

      images_data.map { |image_hash| parse_single_image(image_hash) }
    end

    def parse_single_image(image_hash)
      data = image_hash['data']
      data = data.dup.force_encoding(Encoding::BINARY) if data.respond_to?(:force_encoding)
      Image.new(
        data: data,
        format: image_hash['format'],
        image_index: image_hash['image_index'],
        page_number: image_hash['page_number'],
        width: image_hash['width'],
        height: image_hash['height'],
        colorspace: image_hash['colorspace'],
        bits_per_component: image_hash['bits_per_component'],
        is_mask: image_hash['is_mask'],
        description: image_hash['description'],
        bounding_box: parse_bounding_box(image_hash['bounding_box']),
        ocr_result: image_hash['ocr_result'] ? Result.new(image_hash['ocr_result']) : nil
      )
    end

    def parse_pages(pages_data)
      return nil if pages_data.nil?

      pages_data.map do |page_hash|
        PageContent.new(
          page_number: page_hash['page_number'],
          content: page_hash['content'],
          tables: parse_tables(page_hash['tables']),
          images: parse_images(page_hash['images']),
          hierarchy: parse_page_hierarchy(page_hash['hierarchy']),
          is_blank: page_hash['is_blank'],
          layout_regions: parse_layout_regions(page_hash['layout_regions'])
        )
      end
    end

    def parse_layout_regions(regions_data)
      return nil if regions_data.nil?

      regions_data.map do |region_hash|
        LayoutRegion.new(
          class_name: region_hash['class'],
          confidence: region_hash['confidence']&.to_f,
          bounding_box: parse_element_bounding_box(region_hash['bounding_box']),
          area_fraction: region_hash['area_fraction']&.to_f
        )
      end
    end

    def parse_element_bounding_box(bounding_box_data)
      return nil if bounding_box_data.nil?

      ElementBoundingBox.new(
        x0: bounding_box_data['x0'].to_f,
        y0: bounding_box_data['y0'].to_f,
        x1: bounding_box_data['x1'].to_f,
        y1: bounding_box_data['y1'].to_f
      )
    end

    def parse_page_hierarchy(hierarchy_data)
      return nil if hierarchy_data.nil?

      blocks = (hierarchy_data['blocks'] || []).map do |block_hash|
        HierarchicalBlock.new(
          text: block_hash['text'],
          font_size: block_hash['font_size']&.to_f,
          level: block_hash['level'],
          bbox: block_hash['bbox']
        )
      end

      PageHierarchy.new(
        block_count: hierarchy_data['block_count'] || 0,
        blocks: blocks
      )
    end

    def parse_elements(elements_data)
      return nil if elements_data.nil?

      elements_data.map { |element_hash| parse_element(element_hash) }
    end

    def parse_element(element_hash)
      metadata_hash = element_hash['metadata'] || {}
      coordinates = parse_element_coordinates(metadata_hash['coordinates'])

      metadata = ElementMetadataStruct.new(
        page_number: metadata_hash['page_number'],
        filename: metadata_hash['filename'],
        coordinates: coordinates,
        element_index: metadata_hash['element_index'],
        additional: metadata_hash['additional'] || {}
      )

      ElementStruct.new(
        element_id: element_hash['element_id'],
        element_type: element_hash['element_type'],
        text: element_hash['text'],
        metadata: metadata
      )
    end

    def parse_element_coordinates(coordinates_data)
      return nil if coordinates_data.nil?

      ElementBoundingBox.new(
        x0: coordinates_data['x0'].to_f,
        y0: coordinates_data['y0'].to_f,
        x1: coordinates_data['x1'].to_f,
        y1: coordinates_data['y1'].to_f
      )
    end

    def parse_bounding_box(bounding_box_data)
      return nil if bounding_box_data.nil?

      # If it's already a BoundingBox object, return it
      return bounding_box_data if bounding_box_data.is_a?(BoundingBox)

      # Otherwise parse from hash
      BoundingBox.new(
        x0: bounding_box_data['x0'].to_f,
        y0: bounding_box_data['y0'].to_f,
        x1: bounding_box_data['x1'].to_f,
        y1: bounding_box_data['y1'].to_f
      )
    end

    def parse_ocr_elements(ocr_elements_data)
      return nil if ocr_elements_data.nil?

      ocr_elements_data.map do |element_hash|
        OcrElement.new(
          text: element_hash['text'],
          geometry: parse_ocr_geometry(element_hash['geometry']),
          confidence: parse_ocr_confidence(element_hash['confidence']),
          level: element_hash['level'],
          rotation: parse_ocr_rotation(element_hash['rotation']),
          page_number: element_hash['page_number'],
          parent_id: element_hash['parent_id'],
          backend_metadata: element_hash['backend_metadata']
        )
      end
    end

    def parse_ocr_geometry(data)
      return nil unless data.is_a?(Hash)

      OcrBoundingGeometry.new(
        type: data['type'], left: data['left'], top: data['top'],
        width: data['width'], height: data['height'], points: data['points']
      )
    end

    def parse_ocr_confidence(data)
      return nil unless data.is_a?(Hash)

      OcrConfidence.new(detection: data['detection'], recognition: data['recognition'])
    end

    def parse_ocr_rotation(data)
      return nil unless data.is_a?(Hash)

      OcrRotation.new(angle_degrees: data['angle_degrees'], confidence: data['confidence'])
    end

    def parse_djot_content(djot_data)
      return nil if djot_data.nil?

      DjotContent.new(djot_data)
    end

    def parse_document_structure(document_data)
      return nil if document_data.nil?

      DocumentStructure.new(document_data)
    end

    def parse_extracted_keywords(keywords_data)
      return nil if keywords_data.nil?

      keywords_data.map do |kw_hash|
        Kreuzberg::ExtractedKeyword.new(
          text: kw_hash['text'] || '',
          score: (kw_hash['score'] || 0.0).to_f,
          algorithm: kw_hash['algorithm'] || '',
          positions: kw_hash['positions']
        )
      end
    end

    def parse_processing_warnings(warnings_data)
      return [] if warnings_data.nil?

      warnings_data.map do |w_hash|
        Kreuzberg::ProcessingWarning.new(
          source: w_hash['source'] || '',
          message: w_hash['message'] || ''
        )
      end
    end

    def parse_annotations(annotations_data)
      return nil if annotations_data.nil?

      annotations_data.map { |a_hash| build_annotation(a_hash) }
    end

    def build_annotation(a_hash)
      PdfAnnotation.new(
        annotation_type: a_hash['annotation_type'] || '',
        content: a_hash['content'],
        page_number: a_hash['page_number']&.to_i,
        bounding_box: build_annotation_bbox(a_hash['bounding_box'])
      )
    end

    def build_annotation_bbox(bbox_data)
      return nil if bbox_data.nil?

      PdfAnnotationBoundingBox.new(
        left: bbox_field(bbox_data, 'left', 'x0'),
        top: bbox_field(bbox_data, 'top', 'y0'),
        right: bbox_field(bbox_data, 'right', 'x1'),
        bottom: bbox_field(bbox_data, 'bottom', 'y1')
      )
    end

    def bbox_field(bbox_data, primary_key, fallback_key)
      (bbox_data[primary_key] || bbox_data[fallback_key])&.to_f
    end

    def parse_uris(uris_data)
      return nil if uris_data.nil?

      uris_data.map { |u| build_uri(u) }
    end

    def build_uri(u_hash)
      Struct.new(:url, :label, :page, :kind).new(
        url: u_hash['url'] || '',
        label: u_hash['label'],
        page: u_hash['page']&.to_i,
        kind: u_hash['kind'] || 'hyperlink'
      )
    end

    def parse_children(children_data)
      return nil if children_data.nil?

      children_data.map { |c| build_archive_entry(c) }
    end

    def build_archive_entry(c_hash)
      Struct.new(:path, :mime_type, :result).new(
        path: c_hash['path'] || '',
        mime_type: c_hash['mime_type'] || '',
        result: c_hash['result'] ? self.class.new(c_hash['result']) : nil
      )
    end
  end
  # rubocop:enable Metrics/ClassLength
end
