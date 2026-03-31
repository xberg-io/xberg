defmodule Kreuzberg.ExtractionResult do
  @moduledoc """
  Structure representing the result of a document extraction operation.

  Matches the Rust `ExtractionResult` struct.

  ## Fields

    * `:content` - The main extracted text content
    * `:mime_type` - The MIME type of the processed document
    * `:metadata` - Metadata struct with document information
    * `:tables` - List of extracted tables
    * `:detected_languages` - List of detected language codes
    * `:chunks` - Optional list of text chunks with embeddings
    * `:images` - Optional list of extracted images
    * `:pages` - Optional list of per-page content
    * `:elements` - Optional list of semantic elements
    * `:ocr_elements` - Optional list of OCR elements with positioning and confidence
    * `:djot_content` - Optional rich Djot content structure
    * `:document` - Optional hierarchical document structure
    * `:extracted_keywords` - Optional list of extracted keywords with scores
    * `:quality_score` - Optional quality score for the extraction (0.0 to 1.0)
    * `:processing_warnings` - Optional list of warnings generated during processing
    * `:annotations` - Optional list of PDF annotations (text, highlight, link, etc.)
    * `:uris` - Optional list of URIs extracted from the document
    * `:children` - Optional list of child extraction results (e.g., from archive entries)
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
          elements: list(Kreuzberg.Element.t()) | nil,
          ocr_elements: list(Kreuzberg.OcrElement.t()) | nil,
          djot_content: Kreuzberg.DjotContent.t() | nil,
          document: Kreuzberg.DocumentStructure.t() | nil,
          extracted_keywords: list(Kreuzberg.Keyword.t()) | nil,
          quality_score: float() | nil,
          processing_warnings: list(Kreuzberg.ProcessingWarning.t()),
          annotations: list(Kreuzberg.PdfAnnotation.t()) | nil,
          uris: list(Kreuzberg.Uri.t()) | nil,
          children: list(t()) | nil
        }

  defstruct [
    :detected_languages,
    :chunks,
    :images,
    :pages,
    :elements,
    :ocr_elements,
    :djot_content,
    :document,
    :extracted_keywords,
    :quality_score,
    :annotations,
    :uris,
    :children,
    content: "",
    processing_warnings: [],
    mime_type: "",
    metadata: %Kreuzberg.Metadata{},
    tables: []
  ]

  @doc """
  Creates a new ExtractionResult from extracted data.

  ## Parameters

    * `content` - The extracted text content
    * `mime_type` - The MIME type of the document
    * `metadata` - Document metadata struct or map
    * `tables` - List of extracted table structs or maps
    * `opts` - Optional keyword list with additional fields
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
      elements: normalize_elements(Keyword.get(opts, :elements)),
      ocr_elements: normalize_ocr_elements(Keyword.get(opts, :ocr_elements)),
      djot_content: normalize_djot_content(Keyword.get(opts, :djot_content)),
      document: normalize_document(Keyword.get(opts, :document)),
      extracted_keywords: normalize_keywords(Keyword.get(opts, :extracted_keywords)),
      quality_score: normalize_quality_score(Keyword.get(opts, :quality_score)),
      processing_warnings: normalize_processing_warnings(Keyword.get(opts, :processing_warnings)),
      annotations: normalize_annotations(Keyword.get(opts, :annotations)),
      uris: normalize_uris(Keyword.get(opts, :uris)),
      children: Keyword.get(opts, :children)
    }
  end

  @doc """
  Converts an ExtractionResult struct to a map.
  """
  @spec to_map(t()) :: map()
  def to_map(%__MODULE__{} = result) do
    %{
      "content" => result.content,
      "mime_type" => result.mime_type,
      "metadata" => Kreuzberg.Metadata.to_map(result.metadata),
      "tables" => Enum.map(result.tables, &Kreuzberg.Table.to_map/1),
      "detected_languages" => result.detected_languages,
      "chunks" => maybe_map_list(result.chunks, &Kreuzberg.Chunk.to_map/1),
      "images" => maybe_map_list(result.images, &Kreuzberg.Image.to_map/1),
      "pages" => maybe_map_list(result.pages, &Kreuzberg.Page.to_map/1),
      "elements" => maybe_map_list(result.elements, &Kreuzberg.Element.to_map/1),
      "ocr_elements" => maybe_map_list(result.ocr_elements, &Kreuzberg.OcrElement.to_map/1),
      "djot_content" =>
        case result.djot_content do
          nil -> nil
          %Kreuzberg.DjotContent{} = d -> Kreuzberg.DjotContent.to_map(d)
          other -> other
        end,
      "document" =>
        case result.document do
          nil -> nil
          %Kreuzberg.DocumentStructure{} = doc -> Kreuzberg.DocumentStructure.to_map(doc)
          other -> other
        end,
      "extracted_keywords" =>
        maybe_map_list(result.extracted_keywords, &Kreuzberg.Keyword.to_map/1),
      "quality_score" => result.quality_score,
      "processing_warnings" =>
        maybe_map_list(result.processing_warnings, &Kreuzberg.ProcessingWarning.to_map/1),
      "annotations" => maybe_map_list(result.annotations, &Kreuzberg.PdfAnnotation.to_map/1),
      "uris" => maybe_map_list(result.uris, &Kreuzberg.Uri.to_map/1),
      "children" => maybe_map_list(result.children, &__MODULE__.to_map/1)
    }
  end

  defp maybe_map_list(nil, _fun), do: nil
  defp maybe_map_list(list, fun) when is_list(list), do: Enum.map(list, fun)

  defp normalize_metadata(%Kreuzberg.Metadata{} = metadata), do: metadata
  defp normalize_metadata(map) when is_map(map), do: Kreuzberg.Metadata.from_map(map)
  defp normalize_metadata(nil), do: %Kreuzberg.Metadata{}

  defp normalize_tables(nil), do: []
  defp normalize_tables([]), do: []

  defp normalize_tables(tables) when is_list(tables) do
    Enum.map(tables, fn
      %Kreuzberg.Table{} = table -> table
      map when is_map(map) -> Kreuzberg.Table.from_map(map)
      other -> other
    end)
  end

  defp normalize_chunks(nil), do: nil
  defp normalize_chunks([]), do: []

  defp normalize_chunks(chunks) when is_list(chunks) do
    Enum.map(chunks, fn
      %Kreuzberg.Chunk{} = chunk -> chunk
      map when is_map(map) -> Kreuzberg.Chunk.from_map(map)
      other -> other
    end)
  end

  defp normalize_images(nil), do: nil
  defp normalize_images([]), do: []

  defp normalize_images(images) when is_list(images) do
    Enum.map(images, fn
      %Kreuzberg.Image{} = image -> image
      map when is_map(map) -> Kreuzberg.Image.from_map(map)
      other -> other
    end)
  end

  defp normalize_pages(nil), do: nil
  defp normalize_pages([]), do: []

  defp normalize_pages(pages) when is_list(pages) do
    Enum.map(pages, fn
      %Kreuzberg.Page{} = page -> page
      map when is_map(map) -> Kreuzberg.Page.from_map(map)
      other -> other
    end)
  end

  defp normalize_elements(nil), do: nil
  defp normalize_elements([]), do: []

  defp normalize_elements(elements) when is_list(elements) do
    Enum.map(elements, fn
      %Kreuzberg.Element{} = element -> element
      map when is_map(map) -> Kreuzberg.Element.from_map(map)
      other -> other
    end)
  end

  defp normalize_ocr_elements(nil), do: nil
  defp normalize_ocr_elements([]), do: []

  defp normalize_ocr_elements(elements) when is_list(elements) do
    Enum.map(elements, fn
      %Kreuzberg.OcrElement{} = element -> element
      map when is_map(map) -> Kreuzberg.OcrElement.from_map(map)
      other -> other
    end)
  end

  defp normalize_djot_content(nil), do: nil
  defp normalize_djot_content(%Kreuzberg.DjotContent{} = d), do: d
  defp normalize_djot_content(map) when is_map(map), do: Kreuzberg.DjotContent.from_map(map)

  defp normalize_document(nil), do: nil
  defp normalize_document(%Kreuzberg.DocumentStructure{} = doc), do: doc
  defp normalize_document(map) when is_map(map), do: Kreuzberg.DocumentStructure.from_map(map)

  defp normalize_keywords(nil), do: nil
  defp normalize_keywords([]), do: []

  defp normalize_keywords(keywords) when is_list(keywords) do
    Enum.map(keywords, fn
      %Kreuzberg.Keyword{} = kw -> kw
      map when is_map(map) -> Kreuzberg.Keyword.from_map(map)
      other -> other
    end)
  end

  defp normalize_quality_score(nil), do: nil
  defp normalize_quality_score(score) when is_float(score), do: score
  defp normalize_quality_score(score) when is_integer(score), do: score * 1.0

  defp normalize_processing_warnings(nil), do: []
  defp normalize_processing_warnings([]), do: []

  defp normalize_processing_warnings(warnings) when is_list(warnings) do
    Enum.map(warnings, fn
      %Kreuzberg.ProcessingWarning{} = w -> w
      map when is_map(map) -> Kreuzberg.ProcessingWarning.from_map(map)
      other -> other
    end)
  end

  defp normalize_annotations(nil), do: nil
  defp normalize_annotations([]), do: []

  defp normalize_annotations(annotations) when is_list(annotations) do
    Enum.map(annotations, fn
      %Kreuzberg.PdfAnnotation{} = annotation -> annotation
      map when is_map(map) -> Kreuzberg.PdfAnnotation.from_map(map)
      other -> other
    end)
  end

  defp normalize_uris(nil), do: nil
  defp normalize_uris([]), do: []

  defp normalize_uris(uris) when is_list(uris) do
    Enum.map(uris, fn
      %Kreuzberg.Uri{} = uri -> uri
      map when is_map(map) -> Kreuzberg.Uri.from_map(map)
      other -> other
    end)
  end
end
