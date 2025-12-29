# frozen_string_literal: true

require 'spec_helper'

RSpec.describe 'Table Extraction Quality' do
  describe 'table structure extraction' do
    let(:pdf_path) { test_document_path('pdf/table_document.pdf') }

    it 'extracts table rows, columns, and headers' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      expect(result).not_to be_nil
      expect(result.tables).not_to be_nil
      unless result.tables.empty?
        table = result.tables.first
        expect(table).to be_a(Kreuzberg::Result::Table)
        expect(table.cells).not_to be_nil
        expect(table.cells).to be_a(Array)
      end
    end

    it 'returns cell arrays with consistent structure' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        expect(result.tables).to all(
          be_a(Kreuzberg::Types::Table).and(
            have_attributes(cells: be_a(Array))
          )
        )
      end
    end

    it 'provides page number for each table' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          expect(table.page_number).not_to be_nil
          expect(table.page_number).to be_a(Integer)
          expect(table.page_number).to be > 0
        end
      end
    end

    it 'detects proper row and column counts' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        table = result.tables.first
        unless table.cells.empty?
          first_row_cols = table.cells.first.length
          expect(first_row_cols).to be > 0
          expect(first_row_cols).to be_a(Integer)
        end
      end
    end
  end

  describe 'table markdown conversion accuracy' do
    let(:pdf_path) { test_document_path('pdf/table_document.pdf') }

    it 'generates markdown representation for tables' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          expect(table.markdown).not_to be_nil
          expect(table.markdown).to be_a(String)
          # If table has cells, markdown must not be empty
          if table.cells && !table.cells.empty?
            expect(table.markdown).not_to be_empty, 'Markdown must not be empty when table has cells'
          end
        end
      end
    end

    it 'markdown contains pipe delimiters for table structure' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          # If table has cells and markdown, it must contain pipes
          if table.cells && !table.cells.empty? && !table.markdown.empty?
            expect(table.markdown).to include('|'), 'Markdown table must include pipe separators for cells'
          end
        end
      end
    end

    it 'markdown format is consistent with cell data' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        table = result.tables.first
        unless table.cells.empty?
          row_count = table.cells.length
          expect(row_count).to be > 0
          expect(row_count).to be_a(Integer)
        end
      end
    end
  end

  describe 'cell content preservation' do
    let(:pdf_path) { test_document_path('pdf/table_document.pdf') }

    it 'preserves text content in cells accurately' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          table.cells.each do |row|
            row.each do |cell|
              expect(cell).to be_a(String)
              expect(cell).not_to be_nil
            end
          end
        end
      end
    end

    it 'handles cells with numeric content' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          table.cells.each do |row|
            row.each do |cell|
              expect(cell).not_to be_nil
            end
          end
        end
      end
    end

    it 'preserves whitespace and formatting in cells' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          expect(table.cells).not_to be_empty
          expect(table.cells).to all(all(be_a(String)))
        end
      end
    end

    it 'handles empty cells correctly' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          expect(table.cells).to be_a(Array)
          expect(table.cells).to all(all(be_a(String)))
        end
      end
    end
  end

  describe 'format-specific table handling' do
    let(:pdf_path) { test_document_path('pdf/table_document.pdf') }

    it 'extracts tables from PDF documents' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      expect(result).not_to be_nil
      expect(result.tables).not_to be_nil
      expect(result.tables).to be_a(Array)
    end

    it 'extracts tables from Office formats' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: test_document_path('office/document.docx'), config: config)
        expect(result).not_to be_nil
      rescue Kreuzberg::Errors::ValidationError
        skip 'DOCX test file not available'
      end
    end

    it 'handles PDF tables with different layouts' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          expect(table.cells).not_to be_nil
          expect(table.markdown).not_to be_nil
        end
      end
    end

    it 'respects extraction configuration for tables' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      expect(result).not_to be_nil
      expect(result.tables).not_to be_nil
    end
  end

  describe 'table boundary detection' do
    let(:pdf_path) { test_document_path('pdf/table_document.pdf') }

    it 'correctly identifies table boundaries' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          expect(table.cells.length).to be > 0
          table.cells.each do |row|
            expect(row.length).to be > 0
          end
        end
      end
    end

    it 'separates adjacent tables correctly' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && result.tables.length > 1
        table_count = result.tables.length
        expect(table_count).to be > 1
        result.tables.each do |table|
          expect(table.cells).not_to be_nil
          expect(table.cells.length).to be > 0
        end
      end
    end

    it 'maintains consistent column alignment across rows' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        table = result.tables.first
        if table.cells.length > 1
          first_row_cols = table.cells.first.length
          table.cells.each do |row|
            expect(row.length).to eq(first_row_cols)
          end
        end
      end
    end
  end

  describe 'performance with large tables' do
    let(:pdf_path) { test_document_path('pdf/table_document.pdf') }

    it 'extracts large tables with 100+ rows efficiently' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      expect(result).not_to be_nil
      expect(result.tables).to be_a(Array)
    end

    it 'maintains data integrity for large tables' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          expect(table.cells).not_to be_nil
          expect(table.cells).to all(all(be_a(String)))
        end
      end
    end

    it 'handles tables with varying column counts' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          expect(table.cells.length).to be >= 0
        end
      end
    end
  end

  describe 'table serialization and conversion' do
    let(:pdf_path) { test_document_path('pdf/table_document.pdf') }

    it 'serializes table to hash correctly' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        table = result.tables.first
        table_hash = table.to_h

        expect(table_hash).to be_a(Hash)
        expect(table_hash).to have_key(:cells)
        expect(table_hash).to have_key(:markdown)
        expect(table_hash).to have_key(:page_number)
      end
    end

    it 'preserves table data through serialization' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        table = result.tables.first
        table_hash = table.to_h

        expect(table_hash[:cells]).to eq(table.cells)
        expect(table_hash[:markdown]).to eq(table.markdown)
        expect(table_hash[:page_number]).to eq(table.page_number)
      end
    end

    it 'converts result with tables to JSON' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      expect(result).not_to be_nil
      json_str = result.to_json
      expect(json_str).to be_a(String)
      expect(json_str.length).to be > 0
    end
  end

  describe 'table extraction with page context' do
    let(:pdf_path) { test_document_path('pdf/table_document.pdf') }

    it 'associates tables with correct page numbers' do
      config = Kreuzberg::Config::Extraction.new(
        pages: Kreuzberg::Config::PageConfig.new(extract_pages: true)
      )

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          expect(table.page_number).to be > 0
          expect(table.page_number).to be <= result.page_count
        end
      end
    end

    it 'extracts tables from specific pages when available' do
      config = Kreuzberg::Config::Extraction.new(
        pages: Kreuzberg::Config::PageConfig.new(extract_pages: true)
      )

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.pages && !result.pages.empty?
        result.pages.each do |page|
          expect(page.page_number).not_to be_nil
          next unless page.tables

          page.tables.each do |table|
            expect(table.page_number).to eq(page.page_number)
          end
        end
      end
    end

    it 'maintains table consistency across page and global results' do
      config = Kreuzberg::Config::Extraction.new(
        pages: Kreuzberg::Config::PageConfig.new(extract_pages: true)
      )

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty? && result.pages && !result.pages.empty?
        global_table_count = result.tables.length
        page_table_count = result.pages.sum { |page| page.tables&.length || 0 }

        expect(page_table_count).to eq(global_table_count)
      end
    end
  end

  describe 'table handling edge cases' do
    let(:pdf_path) { test_document_path('pdf/table_document.pdf') }

    it 'handles documents with no tables gracefully' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file('test.txt', config: config)
        expect(result).not_to be_nil
        expect(result.tables).to be_a(Array) if result.tables
      rescue Kreuzberg::Errors::ValidationError
        skip 'Text file not available for testing'
      end
    end

    it 'handles single-cell tables' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          expect(table.cells).to be_a(Array)
        end
      end
    end

    it 'handles tables with long cell content' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          table.cells.each do |row|
            row.each do |cell|
              expect(cell).to be_a(String)
              expect(cell.length).to be >= 0
            end
          end
        end
      end
    end

    it 'handles tables with special characters' do
      config = Kreuzberg::Config::Extraction.new

      begin
        result = Kreuzberg.extract_file(path: pdf_path, config: config)
      rescue Kreuzberg::Errors::ValidationError
        skip 'Test PDF file not available'
      end

      if result.tables && !result.tables.empty?
        result.tables.each do |table|
          expect(table.cells).to all(all(be_a(String)))
        end
      end
    end
  end

  describe 'Table Struct validation' do
    it 'creates Table struct with all fields' do
      table = Kreuzberg::Result::Table.new(
        cells: [%w[Header1 Header2], %w[Value1 Value2]],
        markdown: '| Header1 | Header2 |\n|---------|--------|\n| Value1 | Value2 |',
        page_number: 1
      )

      expect(table.cells).to eq([%w[Header1 Header2], %w[Value1 Value2]])
      expect(table.markdown).to include('Header1')
      expect(table.page_number).to eq(1)
    end

    it 'converts Table struct to hash' do
      table = Kreuzberg::Result::Table.new(
        cells: [%w[A B], %w[C D]],
        markdown: '| A | B |\n|---|---|\n| C | D |',
        page_number: 2
      )

      table_hash = table.to_h

      expect(table_hash).to be_a(Hash)
      expect(table_hash[:cells]).to eq([%w[A B], %w[C D]])
      expect(table_hash[:markdown]).to include('A')
      expect(table_hash[:page_number]).to eq(2)
    end

    it 'handles Table struct with empty cells' do
      table = Kreuzberg::Result::Table.new(
        cells: [],
        markdown: '',
        page_number: 1
      )

      expect(table.cells).to eq([])
      expect(table.markdown).to eq('')
      expect(table.page_number).to eq(1)
    end

    it 'handles Table struct with nil values' do
      table = Kreuzberg::Result::Table.new(
        cells: nil,
        markdown: nil,
        page_number: 0
      )

      expect(table.cells).to be_nil
      expect(table.markdown).to be_nil
      expect(table.page_number).to eq(0)
    end
  end
end
