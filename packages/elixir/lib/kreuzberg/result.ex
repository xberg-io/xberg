defmodule Kreuzberg.ExtractionResult do
  @moduledoc """
  Structure representing the result of a document extraction operation.

  Contains all extracted data from a processed document, including content,
  metadata, tables, detected languages, chunks with embeddings, images with
  OCR results, per-page information, and semantic elements.

  ## Fields

    * `:content` - The main extracted text content as a UTF-8 string
      - Contains the primary textual output from document analysis
      - Cleaned and normalized from the original document
      - May include line breaks and structural markers

    * `:mime_type` - The MIME type of the processed document (e.g., "application/pdf")
      - Used to identify document type and format
      - Common types: "application/pdf", "text/plain", "image/png", etc.
      - Helps downstream processors know how to handle the content

    * `:metadata` - Metadata struct containing document-specific information
      - Proper Kreuzberg.Metadata struct with typed fields
      - Contains title, author, created_date, page_count, etc.
      - Can be an empty struct if no metadata is available

    * `:tables` - List of extracted table structs
      - Each table is a Kreuzberg.Table struct with proper fields
      - Contains cells, headers, markdown, and other table info
      - Empty list [] if no tables found in document

    * `:detected_languages` - List of detected language codes (ISO 639-1 format)
      - Language codes: "en", "de", "fr", "es", "zh", etc.
      - May be nil if language detection is disabled
      - Multiple languages if document contains mixed-language content
      - Example: ["en", "de"] for bilingual document

    * `:chunks` - Optional list of text chunk structs with embeddings
      - nil if chunking/embedding is not enabled
      - Each chunk is a Kreuzberg.Chunk struct with text and embedding
      - Used for semantic search and RAG applications

    * `:images` - Optional list of extracted image structs with OCR results
      - nil if image extraction is disabled
      - Each image is a Kreuzberg.Image struct with format, data, and ocr_text
      - OCR text is result of Tesseract or other OCR backend processing

    * `:pages` - Optional list of per-page content structs
      - nil if page-level extraction is not enabled
      - Each page is a Kreuzberg.Page struct with number, content, and dimensions
      - Useful for documents where position and structure matter

    * `:keywords` - Optional list of extracted keyword maps
      - nil if keyword extraction is disabled
      - Each keyword is a map with "text" and "score" fields
      - Used for document classification, tagging, and search optimization

    * `:elements` - Optional list of semantic element structs
      - nil if element-based extraction is not enabled
      - Each element is a Kreuzberg.Element struct with type and metadata
      - Provides fine-grained semantic classification of document content
      - Compatible with Unstructured.io element format

  ## Examples

      # Basic extraction result
      iex> result = %Kreuzberg.ExtractionResult{
      ...>   content: "Document content",
      ...>   mime_type: "application/pdf",
      ...>   metadata: %Kreuzberg.Metadata{},
      ...>   tables: [],
      ...>   detected_languages: ["en"]
      ...> }
      iex> result.content
      "Document content"

      # Rich extraction with metadata and tables
      iex> result = %Kreuzberg.ExtractionResult{
      ...>   content: "Sales Report 2024\\n\\nQ1: 1M, Q2: 1.2M, Q3: 1.5M",
      ...>   mime_type: "application/pdf",
      ...>   metadata: %Kreuzberg.Metadata{title: "Sales Report"},
      ...>   tables: [%Kreuzberg.Table{headers: ["Quarter", "Amount"]}],
      ...>   detected_languages: ["en"],
      ...>   chunks: nil,
      ...>   images: nil,
      ...>   pages: nil
      ...> }
      iex> result.metadata.title
      "Sales Report"

      # Full extraction with all fields
      iex> result = %Kreuzberg.ExtractionResult{
      ...>   content: "Multi-page document content...",
      ...>   mime_type: "application/pdf",
      ...>   metadata: %Kreuzberg.Metadata{page_count: 5},
      ...>   tables: [%Kreuzberg.Table{cells: [["Data1", "Data2"]]}],
      ...>   detected_languages: ["en", "de"],
      ...>   chunks: [%Kreuzberg.Chunk{text: "chunk1 content"}],
      ...>   images: [%Kreuzberg.Image{format: "png", ocr_text: "Image text"}],
      ...>   pages: [%Kreuzberg.Page{number: 1, content: "Page 1 content"}]
      ...> }
      iex> Enum.count(result.pages)
      1
  """

  @type t :: %__MODULE__{
          content: String.t(),
          mime_type: String.t(),
          metadata: Kreuzberg.Metadata.t(),
          tables: list(Kreuzberg.Table.t()),
          detected_languages: list(String.t()) | nil,
          chunks: list(Kreuzberg.Chunk.t()) | nil,
          images: list(Kreuzberg.Image.t()) | nil,
          pages: list(Kreuzberg.Page.t()) | nil,
          keywords: list(map()) | nil,
          elements: list(Kreuzberg.Element.t()) | nil
        }

  defstruct [
    :content,
    :mime_type,
    :detected_languages,
    :chunks,
    :images,
    :pages,
    :keywords,
    :elements,
    metadata: %Kreuzberg.Metadata{},
    tables: []
  ]

  @doc """
  Creates a new ExtractionResult from extracted data.

  ## Parameters

    * `content` - The extracted text content
    * `mime_type` - The MIME type of the document
    * `metadata` - Document metadata struct or map (defaults to empty Metadata struct)
    * `tables` - List of extracted table structs or maps (defaults to empty list)
    * `opts` - Optional keyword list containing:
      * `:detected_languages` - List of detected language codes
      * `:chunks` - List of chunk structs or maps
      * `:images` - List of image structs or maps
      * `:pages` - List of page structs or maps
      * `:keywords` - List of keyword structs or maps
      * `:elements` - List of element structs or maps

  ## Returns

  An `ExtractionResult` struct with all fields properly typed as structs.

  ## Examples

      iex> Kreuzberg.ExtractionResult.new("text", "text/plain")
      %Kreuzberg.ExtractionResult{
        content: "text",
        mime_type: "text/plain",
        metadata: %Kreuzberg.Metadata{},
        tables: [],
        detected_languages: nil,
        chunks: nil,
        images: nil,
        pages: nil,
        keywords: nil,
        elements: nil
      }

      iex> metadata = %Kreuzberg.Metadata{page_count: 5}
      iex> Kreuzberg.ExtractionResult.new("text", "application/pdf", metadata, [],
      ...>   detected_languages: ["en", "de"])
      %Kreuzberg.ExtractionResult{
        content: "text",
        mime_type: "application/pdf",
        metadata: %Kreuzberg.Metadata{page_count: 5},
        tables: [],
        detected_languages: ["en", "de"],
        chunks: nil,
        images: nil,
        pages: nil,
        keywords: nil,
        elements: nil
      }
  """
  @spec new(
          String.t(),
          String.t(),
          Kreuzberg.Metadata.t() | map(),
          list(Kreuzberg.Table.t() | map()),
          keyword()
        ) :: t()
  def new(content, mime_type, metadata \\ %Kreuzberg.Metadata{}, tables \\ [], opts \\ []) do
    %__MODULE__{
      content: content,
      mime_type: mime_type,
      metadata: normalize_metadata(metadata),
      tables: normalize_tables(tables),
      detected_languages: Keyword.get(opts, :detected_languages),
      chunks: normalize_chunks(Keyword.get(opts, :chunks)),
      images: normalize_images(Keyword.get(opts, :images)),
      pages: normalize_pages(Keyword.get(opts, :pages)),
      keywords: normalize_keywords(Keyword.get(opts, :keywords)),
      elements: normalize_elements(Keyword.get(opts, :elements))
    }
  end

  @doc false
  @spec normalize_metadata(Kreuzberg.Metadata.t() | map()) :: Kreuzberg.Metadata.t()
  defp normalize_metadata(%Kreuzberg.Metadata{} = metadata), do: metadata
  defp normalize_metadata(map) when is_map(map), do: Kreuzberg.Metadata.from_map(map)
  defp normalize_metadata(nil), do: %Kreuzberg.Metadata{}

  @doc false
  @spec normalize_tables(list()) :: list(Kreuzberg.Table.t())
  defp normalize_tables(nil), do: []
  defp normalize_tables([]), do: []

  defp normalize_tables(tables) when is_list(tables) do
    Enum.map(tables, fn
      %Kreuzberg.Table{} = table -> table
      map when is_map(map) -> Kreuzberg.Table.from_map(map)
      other -> other
    end)
  end

  @doc false
  @spec normalize_chunks(list() | nil) :: list(Kreuzberg.Chunk.t()) | nil
  defp normalize_chunks(nil), do: nil
  defp normalize_chunks([]), do: []

  defp normalize_chunks(chunks) when is_list(chunks) do
    Enum.map(chunks, fn
      %Kreuzberg.Chunk{} = chunk -> chunk
      map when is_map(map) -> Kreuzberg.Chunk.from_map(map)
      other -> other
    end)
  end

  @doc false
  @spec normalize_images(list() | nil) :: list(Kreuzberg.Image.t()) | nil
  defp normalize_images(nil), do: nil
  defp normalize_images([]), do: []

  defp normalize_images(images) when is_list(images) do
    Enum.map(images, fn
      %Kreuzberg.Image{} = image -> image
      map when is_map(map) -> Kreuzberg.Image.from_map(map)
      other -> other
    end)
  end

  @doc false
  @spec normalize_pages(list() | nil) :: list(Kreuzberg.Page.t()) | nil
  defp normalize_pages(nil), do: nil
  defp normalize_pages([]), do: []

  defp normalize_pages(pages) when is_list(pages) do
    Enum.map(pages, fn
      %Kreuzberg.Page{} = page -> page
      map when is_map(map) -> Kreuzberg.Page.from_map(map)
      other -> other
    end)
  end

  @doc false
  @spec normalize_keywords(list() | String.t() | nil) :: list(map()) | nil
  defp normalize_keywords(nil), do: nil
  defp normalize_keywords([]), do: []
  defp normalize_keywords(keywords) when is_list(keywords), do: keywords

  defp normalize_keywords(keywords) when is_binary(keywords) do
    keywords
    |> String.split(",")
    |> Enum.map(&String.trim/1)
    |> Enum.reject(&(&1 == ""))
    |> Enum.map(fn text -> %{"text" => text, "score" => 1.0} end)
  end

  @doc false
  @spec normalize_elements(list() | nil) :: list(Kreuzberg.Element.t()) | nil
  defp normalize_elements(nil), do: nil
  defp normalize_elements([]), do: []

  defp normalize_elements(elements) when is_list(elements) do
    Enum.map(elements, fn
      %Kreuzberg.Element{} = element -> element
      map when is_map(map) -> Kreuzberg.Element.from_map(map)
      other -> other
    end)
  end
end
