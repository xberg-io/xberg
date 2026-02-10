# frozen_string_literal: true

# Auto-generated tests for office fixtures.

# rubocop:disable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
require_relative 'spec_helper'

RSpec.describe 'office fixtures' do
  it 'office_bibtex_basic' do
    E2ERuby.run_fixture(
      'office_bibtex_basic',
      'bibtex/comprehensive.bib',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/x-bibtex', 'text/x-bibtex']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_djot_basic' do
    E2ERuby.run_fixture(
      'office_djot_basic',
      'markdown/tables.djot',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['text/x-djot', 'text/djot']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_doc_legacy' do
    E2ERuby.run_fixture(
      'office_doc_legacy',
      'doc/unit_test_lists.doc',
      nil,
      requirements: %w[office],
      notes: 'Requires the office feature.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/msword']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_docbook_basic' do
    E2ERuby.run_fixture(
      'office_docbook_basic',
      'docbook/docbook-reader.docbook',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/docbook+xml', 'text/docbook']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_docx_basic' do
    E2ERuby.run_fixture(
      'office_docx_basic',
      'docx/sample_document.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_docx_equations' do
    E2ERuby.run_fixture(
      'office_docx_equations',
      'docx/equations.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_docx_fake' do
    E2ERuby.run_fixture(
      'office_docx_fake',
      'docx/fake.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_docx_formatting' do
    E2ERuby.run_fixture(
      'office_docx_formatting',
      'docx/unit_test_formatting.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_docx_headers' do
    E2ERuby.run_fixture(
      'office_docx_headers',
      'docx/unit_test_headers.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_docx_lists' do
    E2ERuby.run_fixture(
      'office_docx_lists',
      'docx/unit_test_lists.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_docx_tables' do
    E2ERuby.run_fixture(
      'office_docx_tables',
      'docx/docx_tables.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 50)
      E2ERuby::Assertions.assert_content_contains_all(result, ['Simple uniform table', 'Nested Table', 'merged cells', 'Header Col'])
      E2ERuby::Assertions.assert_table_count(result, 1, nil)
    end
  end

  it 'office_epub_basic' do
    E2ERuby.run_fixture(
      'office_epub_basic',
      'epub/features.epub',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/epub+zip']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 50)
    end
  end

  it 'office_fb2_basic' do
    E2ERuby.run_fixture(
      'office_fb2_basic',
      'fictionbook/basic.fb2',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/x-fictionbook+xml']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_fictionbook_basic' do
    E2ERuby.run_fixture(
      'office_fictionbook_basic',
      'fictionbook/basic.fb2',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/x-fictionbook+xml', 'application/x-fictionbook']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_jats_basic' do
    E2ERuby.run_fixture(
      'office_jats_basic',
      'jats/sample_article.jats',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/x-jats+xml', 'text/jats']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_jupyter_basic' do
    E2ERuby.run_fixture(
      'office_jupyter_basic',
      'jupyter/rank.ipynb',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/x-ipynb+json']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_latex_basic' do
    E2ERuby.run_fixture(
      'office_latex_basic',
      'latex/basic_sections.tex',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/x-latex', 'text/x-latex']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_markdown_basic' do
    E2ERuby.run_fixture(
      'office_markdown_basic',
      'markdown/comprehensive.md',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['text/markdown']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_ods_basic' do
    E2ERuby.run_fixture(
      'office_ods_basic',
      'data_formats/test_01.ods',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.oasis.opendocument.spreadsheet']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_odt_bold' do
    E2ERuby.run_fixture(
      'office_odt_bold',
      'odt/bold.odt',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.oasis.opendocument.text']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_odt_list' do
    E2ERuby.run_fixture(
      'office_odt_list',
      'odt/unorderedList.odt',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.oasis.opendocument.text']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 30)
      E2ERuby::Assertions.assert_content_contains_any(result, ['list item', 'New level', 'Pushed us'])
    end
  end

  it 'office_odt_simple' do
    E2ERuby.run_fixture(
      'office_odt_simple',
      'odt/simple.odt',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.oasis.opendocument.text']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 50)
      E2ERuby::Assertions.assert_content_contains_any(result, ['favorite things', 'Parrots', 'Analysis'])
    end
  end

  it 'office_odt_table' do
    E2ERuby.run_fixture(
      'office_odt_table',
      'odt/table.odt',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.oasis.opendocument.text']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
      E2ERuby::Assertions.assert_table_count(result, 1, nil)
    end
  end

  it 'office_opml_basic' do
    E2ERuby.run_fixture(
      'office_opml_basic',
      'opml/outline.opml',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/xml+opml', 'text/x-opml', 'application/x-opml+xml']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_org_basic' do
    E2ERuby.run_fixture(
      'office_org_basic',
      'org/comprehensive.org',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['text/x-org', 'text/org']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_ppsx_slideshow' do
    E2ERuby.run_fixture(
      'office_ppsx_slideshow',
      'pptx/sample.ppsx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.presentationml.slideshow']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_ppt_legacy' do
    E2ERuby.run_fixture(
      'office_ppt_legacy',
      'ppt/simple.ppt',
      nil,
      requirements: %w[office],
      notes: 'Requires the office feature.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.ms-powerpoint']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_pptx_basic' do
    E2ERuby.run_fixture(
      'office_pptx_basic',
      'pptx/simple.pptx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.presentationml.presentation']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 50)
    end
  end

  it 'office_pptx_images' do
    E2ERuby.run_fixture(
      'office_pptx_images',
      'pptx/powerpoint_with_image.pptx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.presentationml.presentation']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_pptx_pitch_deck' do
    E2ERuby.run_fixture(
      'office_pptx_pitch_deck',
      'pptx/pitch_deck_presentation.pptx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.presentationml.presentation']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 100)
    end
  end

  it 'office_rst_basic' do
    E2ERuby.run_fixture(
      'office_rst_basic',
      'rst/restructured_text.rst',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['text/x-rst', 'text/prs.fallenstein.rst']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_rtf_basic' do
    E2ERuby.run_fixture(
      'office_rtf_basic',
      'rtf/extraction_test.rtf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/rtf', 'text/rtf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_typst_basic' do
    E2ERuby.run_fixture(
      'office_typst_basic',
      'typst/headings.typ',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/x-typst', 'text/x-typst']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_xls_legacy' do
    E2ERuby.run_fixture(
      'office_xls_legacy',
      'xls/test_excel.xls',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.ms-excel']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_xlsx_basic' do
    E2ERuby.run_fixture(
      'office_xlsx_basic',
      'xlsx/stanley_cups.xlsx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.spreadsheetml.sheet']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 100)
      E2ERuby::Assertions.assert_content_contains_all(result, ['Team', 'Location', 'Stanley Cups'])
      E2ERuby::Assertions.assert_table_count(result, 1, nil)
      E2ERuby::Assertions.assert_metadata_expectation(result, 'sheet_count', { gte: 2 })
      E2ERuby::Assertions.assert_metadata_expectation(result, 'sheet_names', { contains: ['Stanley Cups'] })
    end
  end

  it 'office_xlsx_multi_sheet' do
    E2ERuby.run_fixture(
      'office_xlsx_multi_sheet',
      'xlsx/excel_multi_sheet.xlsx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.spreadsheetml.sheet']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
      E2ERuby::Assertions.assert_metadata_expectation(result, 'sheet_count', { gte: 2 })
    end
  end

  it 'office_xlsx_office_example' do
    E2ERuby.run_fixture(
      'office_xlsx_office_example',
      'xlsx/test_01.xlsx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.spreadsheetml.sheet']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end
end
# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
