# frozen_string_literal: true

# Auto-generated tests for ocr fixtures.

# rubocop:disable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
require_relative 'spec_helper'

RSpec.describe 'ocr fixtures' do
  it 'ocr_image_hello_world' do
    E2ERuby.run_fixture(
      'ocr_image_hello_world',
      'images/test_hello_world.png',
      { force_ocr: true, ocr: { backend: 'tesseract', language: 'eng' } },
      requirements: %w[tesseract tesseract],
      notes: 'Requires Tesseract OCR for image text extraction.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['image/png']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 5)
      E2ERuby::Assertions.assert_content_contains_any(result, %w[hello world])
    end
  end

  it 'ocr_image_no_text' do
    E2ERuby.run_fixture(
      'ocr_image_no_text',
      'images/flower_no_text.jpg',
      { force_ocr: true, ocr: { backend: 'tesseract', language: 'eng' } },
      requirements: %w[tesseract tesseract],
      notes: 'Skip when Tesseract is unavailable.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['image/jpeg']
      )
      E2ERuby::Assertions.assert_max_content_length(result, 200)
    end
  end

  it 'ocr_paddle_confidence_filter' do
    E2ERuby.run_fixture(
      'ocr_paddle_confidence_filter',
      'images/ocr_image.jpg',
      { force_ocr: true, ocr: { backend: 'paddle-ocr', language: 'en', paddle_ocr_config: { min_confidence: 80 } } },
      requirements: %w[paddle-ocr paddle-ocr onnxruntime],
      notes: 'Tests confidence threshold filtering with PaddleOCR',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['image/jpeg']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 1)
    end
  end

  it 'ocr_paddle_image_chinese' do
    E2ERuby.run_fixture(
      'ocr_paddle_image_chinese',
      'images/chi_sim_image.jpeg',
      { force_ocr: true, ocr: { backend: 'paddle-ocr', language: 'ch' } },
      requirements: %w[paddle-ocr paddle-ocr onnxruntime],
      notes: 'Requires PaddleOCR with Chinese models',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['image/jpeg']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 1)
    end
  end

  it 'ocr_paddle_image_english' do
    E2ERuby.run_fixture(
      'ocr_paddle_image_english',
      'images/test_hello_world.png',
      { force_ocr: true, ocr: { backend: 'paddle-ocr', language: 'en' } },
      requirements: %w[paddle-ocr paddle-ocr onnxruntime],
      notes: 'Requires PaddleOCR with ONNX Runtime',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['image/png']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 5)
      E2ERuby::Assertions.assert_content_contains_any(result, %w[hello Hello world World])
    end
  end

  it 'ocr_paddle_markdown' do
    E2ERuby.run_fixture(
      'ocr_paddle_markdown',
      'images/test_hello_world.png',
      { force_ocr: true, ocr: { backend: 'paddle-ocr', language: 'en', paddle_ocr_config: { output_format: 'markdown' } } },
      requirements: %w[paddle-ocr paddle-ocr onnxruntime],
      notes: 'Tests markdown output format parity with Tesseract',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['image/png']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 5)
      E2ERuby::Assertions.assert_content_contains_any(result, %w[hello Hello world World])
    end
  end

  it 'ocr_paddle_pdf_scanned' do
    E2ERuby.run_fixture(
      'ocr_paddle_pdf_scanned',
      'pdf/ocr_test.pdf',
      { force_ocr: true, ocr: { backend: 'paddle-ocr', language: 'en' } },
      requirements: %w[paddle-ocr paddle-ocr onnxruntime],
      notes: 'Requires PaddleOCR with ONNX Runtime',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
      E2ERuby::Assertions.assert_content_contains_any(result, %w[Docling Markdown JSON])
    end
  end

  it 'ocr_paddle_structured' do
    E2ERuby.run_fixture(
      'ocr_paddle_structured',
      'images/test_hello_world.png',
      { force_ocr: true, ocr: { backend: 'paddle-ocr', element_config: { include_elements: true }, language: 'en' } },
      requirements: %w[paddle-ocr paddle-ocr onnxruntime],
      notes: 'Tests structured output with bbox/confidence preservation',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['image/png']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 5)
      E2ERuby::Assertions.assert_ocr_elements(result, has_elements: true, elements_have_geometry: true, elements_have_confidence: true)
    end
  end

  it 'ocr_paddle_table_detection' do
    E2ERuby.run_fixture(
      'ocr_paddle_table_detection',
      'images/simple_table.png',
      { force_ocr: true, ocr: { backend: 'paddle-ocr', language: 'en', paddle_ocr_config: { enable_table_detection: true } } },
      requirements: %w[paddle-ocr paddle-ocr onnxruntime],
      notes: 'Tests table detection capability with PaddleOCR',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['image/png']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
      E2ERuby::Assertions.assert_table_count(result, 1, nil)
    end
  end

  it 'ocr_pdf_image_only_german' do
    E2ERuby.run_fixture(
      'ocr_pdf_image_only_german',
      'pdf/image_only_german_pdf.pdf',
      { force_ocr: true, ocr: { backend: 'tesseract', language: 'eng' } },
      requirements: %w[tesseract tesseract],
      notes: 'Skip if OCR backend unavailable.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
      E2ERuby::Assertions.assert_metadata_expectation(result, 'format_type', { eq: 'pdf' })
    end
  end

  it 'ocr_pdf_rotated_90' do
    E2ERuby.run_fixture(
      'ocr_pdf_rotated_90',
      'pdf/ocr_test_rotated_90.pdf',
      { force_ocr: true, ocr: { backend: 'tesseract', language: 'eng' } },
      requirements: %w[tesseract tesseract],
      notes: 'Skip automatically when OCR backend is missing.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'ocr_pdf_tesseract' do
    E2ERuby.run_fixture(
      'ocr_pdf_tesseract',
      'pdf/ocr_test.pdf',
      { force_ocr: true, ocr: { backend: 'tesseract', language: 'eng' } },
      requirements: %w[tesseract tesseract],
      notes: 'Skip automatically if OCR backend is unavailable.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
      E2ERuby::Assertions.assert_content_contains_any(result, %w[Docling Markdown JSON])
    end
  end
end
# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
