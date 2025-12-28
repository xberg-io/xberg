defmodule Kreuzberg.ExtractionResult do
  @moduledoc """
  Structure representing the result of a document extraction operation.

  Contains all extracted data from a processed document, including content,
  metadata, tables, detected languages, chunks with embeddings, images with
  OCR results, and per-page information.

  ## Fields

    * `:content` - The main extracted text content as a UTF-8 string
      - Contains the primary textual output from document analysis
      - Cleaned and normalized from the original document
      - May include line breaks and structural markers

    * `:mime_type` - The MIME type of the processed document (e.g., "application/pdf")
      - Used to identify document type and format
      - Common types: "application/pdf", "text/plain", "image/png", etc.
      - Helps downstream processors know how to handle the content

    * `:metadata` - Metadata structure containing document-specific information
      - Map format: %{"key" => "value"}
      - Common keys: "title", "author", "created_date", "page_count", "file_size"
      - Can be empty (%{}) if no metadata is available
      - Example: %{"pages" => 5, "title" => "Report.pdf"}

    * `:tables` - List of extracted table structures
      - Each table is a map with "rows" and "columns" keys
      - Row data is nested lists: [["cell1", "cell2"], ["cell3", "cell4"]]
      - Column metadata may include headers and alignment information
      - Empty list [] if no tables found in document

    * `:detected_languages` - List of detected language codes (ISO 639-1 format)
      - Language codes: "en", "de", "fr", "es", "zh", etc.
      - May be nil if language detection is disabled
      - Multiple languages if document contains mixed-language content
      - Example: ["en", "de"] for bilingual document

    * `:chunks` - Optional list of text chunks with embeddings and metadata
      - nil if chunking/embedding is not enabled
      - Each chunk: %{"text" => "chunk content", "embedding" => [...], "metadata" => %{}}
      - Used for semantic search and RAG applications
      - Chunk size depends on embedding model and configuration

    * `:images` - Optional list of extracted images with nested OCR results
      - nil if image extraction is disabled
      - Each image: %{"data" => binary, "format" => "png", "ocr_text" => "extracted text"}
      - OCR text is result of Tesseract or other OCR backend processing
      - Format can be "png", "jpeg", "webp" depending on extraction settings

    * `:pages` - Optional list of per-page content when page extraction is enabled
      - nil if page-level extraction is not enabled
      - Each page: %{"number" => 1, "content" => "page text", "height" => 11.0, "width" => 8.5}
      - Page dimensions in inches or centimeters
      - Useful for documents where position and structure matter

  ## Examples

      # Basic extraction result
      iex> result = %Kreuzberg.ExtractionResult{
      ...>   content: "Document content",
      ...>   mime_type: "application/pdf",
      ...>   metadata: %{},
      ...>   tables: [],
      ...>   detected_languages: ["en"]
      ...> }
      iex> result.content
      "Document content"

      # Rich extraction with metadata and tables
      iex> result = %Kreuzberg.ExtractionResult{
      ...>   content: "Sales Report 2024\\n\\nQ1: 1M, Q2: 1.2M, Q3: 1.5M",
      ...>   mime_type: "application/pdf",
      ...>   metadata: %{"title" => "Sales Report", "year" => "2024", "pages" => 3},
      ...>   tables: [%{"headers" => ["Quarter", "Amount"],
      ...>             "rows" => [["Q1", "1M"], ["Q2", "1.2M"], ["Q3", "1.5M"]]}],
      ...>   detected_languages: ["en"],
      ...>   chunks: nil,
      ...>   images: nil,
      ...>   pages: nil
      ...> }
      iex> result.metadata["title"]
      "Sales Report"

      # Full extraction with all fields
      iex> result = %Kreuzberg.ExtractionResult{
      ...>   content: "Multi-page document content...",
      ...>   mime_type: "application/pdf",
      ...>   metadata: %{"pages" => 5, "author" => "John Doe"},
      ...>   tables: [%{"rows" => [["Data1", "Data2"]]}],
      ...>   detected_languages: ["en", "de"],
      ...>   chunks: [%{"text" => "chunk1 content", "embedding" => [...]}],
      ...>   images: [%{"data" => <<...>>, "format" => "png", "ocr_text" => "Image text"}],
      ...>   pages: [%{"number" => 1, "content" => "Page 1 content"}]
      ...> }
      iex> Enum.count(result.pages)
      1
  """

  @type t :: %__MODULE__{
          content: String.t(),
          mime_type: String.t(),
          metadata: map(),
          tables: list(map()),
          detected_languages: list(String.t()) | nil,
          chunks: list(map()) | nil,
          images: list(map()) | nil,
          pages: list(map()) | nil
        }

  defstruct [
    :content,
    :mime_type,
    :metadata,
    :tables,
    :detected_languages,
    :chunks,
    :images,
    :pages
  ]

  @doc """
  Creates a new ExtractionResult from extracted data.

  ## Parameters

    * `content` - The extracted text content
    * `mime_type` - The MIME type of the document
    * `metadata` - Document metadata (defaults to empty map)
    * `tables` - List of extracted tables (defaults to empty list)
    * `opts` - Optional keyword list containing:
      * `:detected_languages` - List of detected language codes
      * `:chunks` - List of text chunks with embeddings
      * `:images` - List of extracted images
      * `:pages` - List of per-page content

  ## Returns

  An `ExtractionResult` struct with all fields populated.

  ## Examples

      iex> Kreuzberg.ExtractionResult.new("text", "text/plain")
      %Kreuzberg.ExtractionResult{
        content: "text",
        mime_type: "text/plain",
        metadata: %{},
        tables: [],
        detected_languages: nil,
        chunks: nil,
        images: nil,
        pages: nil
      }

      iex> Kreuzberg.ExtractionResult.new("text", "application/pdf", %{"pages" => 5}, [],
      ...>   detected_languages: ["en", "de"])
      %Kreuzberg.ExtractionResult{
        content: "text",
        mime_type: "application/pdf",
        metadata: %{"pages" => 5},
        tables: [],
        detected_languages: ["en", "de"],
        chunks: nil,
        images: nil,
        pages: nil
      }
  """
  @spec new(
          String.t(),
          String.t(),
          map(),
          list(map()),
          keyword()
        ) :: t()
  def new(content, mime_type, metadata \\ %{}, tables \\ [], opts \\ []) do
    %__MODULE__{
      content: content,
      mime_type: mime_type,
      metadata: metadata,
      tables: tables,
      detected_languages: Keyword.get(opts, :detected_languages),
      chunks: Keyword.get(opts, :chunks),
      images: Keyword.get(opts, :images),
      pages: Keyword.get(opts, :pages)
    }
  end
end
