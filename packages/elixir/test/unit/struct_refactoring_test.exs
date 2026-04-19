defmodule KreuzbergTest.Unit.StructRefactoringTest do
  @moduledoc """
  Tests for the Elixir struct refactoring to use proper idiomatic types.

  Validates that ExtractionResult and nested types properly use structs
  instead of raw maps, ensuring type safety and idiomatic Elixir code.
  """

  use ExUnit.Case

  describe "Metadata struct" do
    test "creates metadata struct from map" do
      map = %{
        "title" => "Report 2024",
        "authors" => ["John Doe"],
        "created_at" => "2024-01-15T10:30:00Z"
      }

      metadata = Kreuzberg.Metadata.from_map(map)

      assert metadata.title == "Report 2024"
      assert metadata.authors == ["John Doe"]
      assert metadata.created_at == "2024-01-15T10:30:00Z"
      assert is_struct(metadata, Kreuzberg.Metadata)
    end

    test "converts metadata struct to map" do
      metadata = %Kreuzberg.Metadata{
        title: "Report",
        authors: ["Jane"]
      }

      map = Kreuzberg.Metadata.to_map(metadata)

      assert map["title"] == "Report"
      assert map["authors"] == ["Jane"]
      assert is_map(map)
    end

    test "handles empty metadata struct" do
      metadata = %Kreuzberg.Metadata{}

      assert metadata.title == nil
      assert metadata.authors == nil
      assert is_struct(metadata, Kreuzberg.Metadata)
    end
  end

  describe "Table struct" do
    test "creates table struct from map" do
      map = %{
        "cells" => [["A", "B"], ["1", "2"]],
        "markdown" => "| A | B |\n|---|---|\n| 1 | 2 |"
      }

      table = Kreuzberg.Table.from_map(map)

      assert table.cells == [["A", "B"], ["1", "2"]]
      assert table.markdown =~ "|"
      assert is_struct(table, Kreuzberg.Table)
    end

    test "converts table struct to map" do
      table = %Kreuzberg.Table{
        cells: [["X", "Y"]]
      }

      map = Kreuzberg.Table.to_map(table)

      assert map["cells"] == [["X", "Y"]]
    end

    test "calculates row and column counts" do
      table = %Kreuzberg.Table{
        cells: [["A", "B"], ["1", "2"], ["3", "4"]]
      }

      assert Kreuzberg.Table.row_count(table) == 3
      assert Kreuzberg.Table.column_count(table) == 2
    end

    test "handles empty table" do
      table = %Kreuzberg.Table{}

      assert Kreuzberg.Table.row_count(table) == 0
      assert Kreuzberg.Table.column_count(table) == 0
    end
  end

  describe "Chunk struct" do
    test "creates chunk struct with new/2" do
      chunk = Kreuzberg.Chunk.new("chunk text", embedding: [0.1, 0.2], metadata: %{"page" => 1})

      assert chunk.content == "chunk text"
      assert chunk.embedding == [0.1, 0.2]
      assert chunk.metadata == %{"page" => 1}
      assert is_struct(chunk, Kreuzberg.Chunk)
    end

    test "creates chunk from map" do
      map = %{
        "content" => "content",
        "embedding" => [0.3, 0.4, 0.5],
        "metadata" => %{"token_count" => 15}
      }

      chunk = Kreuzberg.Chunk.from_map(map)

      assert chunk.content == "content"
      assert chunk.embedding == [0.3, 0.4, 0.5]
      assert chunk.metadata.token_count == 15
    end

    test "converts chunk to map" do
      chunk = %Kreuzberg.Chunk{
        content: "text",
        embedding: [0.1, 0.2]
      }

      map = Kreuzberg.Chunk.to_map(chunk)

      assert map["content"] == "text"
      assert map["embedding"] == [0.1, 0.2]
    end
  end

  describe "Image struct" do
    test "creates image struct with new/2" do
      image = Kreuzberg.Image.new("png", width: 1024, height: 768)

      assert image.format == "png"
      assert image.width == 1024
      assert image.height == 768
      assert is_struct(image, Kreuzberg.Image)
    end

    test "creates image from map" do
      map = %{
        "format" => "jpeg",
        "width" => 1920,
        "height" => 1080,
        "ocr_result" => %{"content" => "extracted text", "mime_type" => "text/plain"}
      }

      image = Kreuzberg.Image.from_map(map)

      assert image.format == "jpeg"
      assert image.width == 1920
      assert image.ocr_result.content == "extracted text"
    end

    test "converts image to map" do
      image = %Kreuzberg.Image{
        format: "webp",
        width: 800
      }

      map = Kreuzberg.Image.to_map(image)

      assert map["format"] == "webp"
      assert map["width"] == 800
    end

    test "checks if image has data" do
      image_with_data = %Kreuzberg.Image{
        format: "png",
        data: <<137, 80, 78, 71>>
      }

      image_without_data = %Kreuzberg.Image{format: "png"}

      assert Kreuzberg.Image.has_data?(image_with_data)
      refute Kreuzberg.Image.has_data?(image_without_data)
    end

    test "calculates aspect ratio" do
      image = %Kreuzberg.Image{width: 1920, height: 1080}

      ratio = image.width / image.height

      assert is_float(ratio)
      assert abs(ratio - 1.777) < 0.01
    end

    test "returns nil for aspect ratio without dimensions" do
      image = %Kreuzberg.Image{format: "png"}

      ratio = if image.width && image.height, do: image.width / image.height, else: nil

      assert ratio == nil
    end
  end

  describe "LayoutRegion struct" do
    test "creates layout region from map" do
      map = %{
        "class" => "picture",
        "confidence" => 0.95,
        "bounding_box" => %{"x0" => 10.0, "y0" => 20.0, "x1" => 200.0, "y1" => 300.0},
        "area_fraction" => 0.3
      }

      region = Kreuzberg.LayoutRegion.from_map(map)

      assert region.class == "picture"
      assert region.confidence == 0.95
      assert region.bounding_box == %{"x0" => 10.0, "y0" => 20.0, "x1" => 200.0, "y1" => 300.0}
      assert region.area_fraction == 0.3
      assert is_struct(region, Kreuzberg.LayoutRegion)
    end

    test "converts layout region to map" do
      region = %Kreuzberg.LayoutRegion{
        class: "table",
        confidence: 0.88,
        bounding_box: nil,
        area_fraction: 0.15
      }

      map = Kreuzberg.LayoutRegion.to_map(region)

      assert map["class"] == "table"
      assert map["confidence"] == 0.88
      assert map["bounding_box"] == nil
      assert map["area_fraction"] == 0.15
    end

    test "uses default values for missing fields" do
      region = Kreuzberg.LayoutRegion.from_map(%{})

      assert region.class == ""
      assert region.confidence == 0.0
      assert region.bounding_box == nil
      assert region.area_fraction == 0.0
    end

    test "coerces integer confidence and area_fraction to float" do
      region = Kreuzberg.LayoutRegion.from_map(%{"confidence" => 1, "area_fraction" => 0})

      assert is_float(region.confidence)
      assert is_float(region.area_fraction)
      assert region.confidence == 1.0
      assert region.area_fraction == 0.0
    end
  end

  describe "Page struct" do
    test "creates page struct with from_map" do
      page = Kreuzberg.Page.from_map(%{"page_number" => 1, "content" => "Page content"})

      assert page.page_number == 1
      assert page.content == "Page content"
      assert is_struct(page, Kreuzberg.Page)
    end

    test "creates page from map" do
      map = %{
        "page_number" => 2,
        "content" => "page text"
      }

      page = Kreuzberg.Page.from_map(map)

      assert page.page_number == 2
      assert page.content == "page text"
    end

    test "converts page to map" do
      page = %Kreuzberg.Page{
        page_number: 3,
        content: "content"
      }

      map = Kreuzberg.Page.to_map(page)

      assert map["page_number"] == 3
      assert map["content"] == "content"
    end

    test "layout_regions defaults to nil when absent from map" do
      page = Kreuzberg.Page.from_map(%{"page_number" => 0, "content" => ""})

      assert page.layout_regions == nil
    end

    test "normalizes layout_regions from maps on from_map" do
      map = %{
        "page_number" => 0,
        "content" => "text",
        "layout_regions" => [
          %{"class" => "picture", "confidence" => 0.9, "area_fraction" => 0.25},
          %{"class" => "text", "confidence" => 0.98, "area_fraction" => 0.6}
        ]
      }

      page = Kreuzberg.Page.from_map(map)

      assert is_list(page.layout_regions)
      assert length(page.layout_regions) == 2

      [first, second] = page.layout_regions

      assert is_struct(first, Kreuzberg.LayoutRegion)
      assert first.class == "picture"
      assert first.confidence == 0.9

      assert is_struct(second, Kreuzberg.LayoutRegion)
      assert second.class == "text"
    end

    test "preserves existing LayoutRegion structs in normalize" do
      region = %Kreuzberg.LayoutRegion{class: "diagram", confidence: 0.75, area_fraction: 0.1}

      page =
        Kreuzberg.Page.from_map(%{
          "page_number" => 1,
          "content" => "text",
          "layout_regions" => [region]
        })

      assert [^region] = page.layout_regions
    end

    test "serializes layout_regions to maps in to_map" do
      region = %Kreuzberg.LayoutRegion{class: "table", confidence: 0.85, area_fraction: 0.2}
      page = %Kreuzberg.Page{page_number: 0, content: "text", layout_regions: [region]}

      map = Kreuzberg.Page.to_map(page)

      assert [region_map] = map["layout_regions"]
      assert region_map["class"] == "table"
      assert region_map["confidence"] == 0.85
    end

    test "serializes nil layout_regions as nil in to_map" do
      page = %Kreuzberg.Page{page_number: 0, content: "text", layout_regions: nil}

      map = Kreuzberg.Page.to_map(page)

      assert map["layout_regions"] == nil
    end

    test "serializes empty layout_regions list in to_map" do
      page = %Kreuzberg.Page{page_number: 0, content: "text", layout_regions: []}

      map = Kreuzberg.Page.to_map(page)

      assert map["layout_regions"] == []
    end
  end

  describe "ExtractionResult struct with nested structs" do
    test "creates result with struct fields" do
      metadata = %Kreuzberg.Metadata{title: "Report"}
      table = %Kreuzberg.Table{cells: [["Col1", "Col2"]]}

      result =
        Kreuzberg.ExtractionResult.new(
          "content",
          "application/pdf",
          metadata,
          [table]
        )

      assert result.content == "content"
      assert result.mime_type == "application/pdf"
      assert is_struct(result.metadata, Kreuzberg.Metadata)
      assert result.metadata.title == "Report"
      assert length(result.tables) == 1
      assert is_struct(Enum.at(result.tables, 0), Kreuzberg.Table)
    end

    test "converts maps to structs automatically" do
      metadata_map = %{"title" => "Report"}
      table_map = %{"cells" => [["A", "B"]]}

      result =
        Kreuzberg.ExtractionResult.new(
          "text",
          "text/plain",
          metadata_map,
          [table_map]
        )

      assert is_struct(result.metadata, Kreuzberg.Metadata)
      assert result.metadata.title == "Report"
      assert is_struct(Enum.at(result.tables, 0), Kreuzberg.Table)
    end

    test "normalizes chunks to structs" do
      chunk_map = %{"content" => "chunk", "embedding" => [0.1, 0.2]}

      result =
        Kreuzberg.ExtractionResult.new(
          "content",
          "text/plain",
          %Kreuzberg.Metadata{},
          [],
          chunks: [chunk_map]
        )

      assert result.chunks != nil
      chunk = Enum.at(result.chunks, 0)
      assert is_struct(chunk, Kreuzberg.Chunk)
      assert chunk.content == "chunk"
    end

    test "normalizes images to structs" do
      image_map = %{"format" => "png", "width" => 800}

      result =
        Kreuzberg.ExtractionResult.new(
          "content",
          "text/plain",
          %Kreuzberg.Metadata{},
          [],
          images: [image_map]
        )

      assert result.images != nil
      image = Enum.at(result.images, 0)
      assert is_struct(image, Kreuzberg.Image)
      assert image.format == "png"
    end

    test "normalizes pages to structs" do
      page_map = %{"page_number" => 1, "content" => "page text"}

      result =
        Kreuzberg.ExtractionResult.new(
          "content",
          "text/plain",
          %Kreuzberg.Metadata{},
          [],
          pages: [page_map]
        )

      assert result.pages != nil
      page = Enum.at(result.pages, 0)
      assert is_struct(page, Kreuzberg.Page)
      assert page.page_number == 1
    end

    test "handles empty metadata default" do
      result = Kreuzberg.ExtractionResult.new("content", "text/plain")

      assert is_struct(result.metadata, Kreuzberg.Metadata)
      assert result.tables == []
    end

    test "pattern matches on nested structs" do
      metadata = %Kreuzberg.Metadata{title: "Test"}
      result = Kreuzberg.ExtractionResult.new("content", "text/plain", metadata)

      assert %Kreuzberg.ExtractionResult{
               metadata: %Kreuzberg.Metadata{title: title}
             } = result

      assert title == "Test"
    end
  end

  describe "ExtractionConfig struct" do
    test "enforces struct type in to_map" do
      config = %Kreuzberg.ExtractionConfig{
        use_cache: true,
        chunking: %{"size" => 512}
      }

      map = Kreuzberg.ExtractionConfig.to_map(config)

      assert map["use_cache"] == true
      assert map["chunking"] == %{"size" => 512}
      assert is_map(map)
    end

    test "accepts raw maps in to_map" do
      raw_map = %{"use_cache" => false}

      result = Kreuzberg.ExtractionConfig.to_map(raw_map)
      assert result == raw_map
    end

    test "handles nil config in to_map" do
      assert Kreuzberg.ExtractionConfig.to_map(nil) == nil
    end

    test "validates correct struct types" do
      config = %Kreuzberg.ExtractionConfig{
        use_cache: true,
        enable_quality_processing: false
      }

      {:ok, validated} = Kreuzberg.ExtractionConfig.validate(config)

      assert validated.use_cache == true
      assert validated.enable_quality_processing == false
    end
  end

  describe "Type safety and idiomatic Elixir patterns" do
    test "all result nested fields are structs" do
      result = %Kreuzberg.ExtractionResult{
        content: "text",
        mime_type: "text/plain",
        metadata: %Kreuzberg.Metadata{},
        tables: [],
        detected_languages: ["en"],
        chunks: [%Kreuzberg.Chunk{content: "chunk"}],
        images: [%Kreuzberg.Image{format: "png"}],
        pages: [%Kreuzberg.Page{page_number: 1, content: "page"}]
      }

      assert is_struct(result.metadata, Kreuzberg.Metadata)

      Enum.each(result.tables, fn table ->
        assert is_struct(table, Kreuzberg.Table)
      end)

      if result.chunks do
        Enum.each(result.chunks, fn chunk ->
          assert is_struct(chunk, Kreuzberg.Chunk)
        end)
      end

      if result.images do
        Enum.each(result.images, fn image ->
          assert is_struct(image, Kreuzberg.Image)
        end)
      end

      if result.pages do
        Enum.each(result.pages, fn page ->
          assert is_struct(page, Kreuzberg.Page)
        end)
      end
    end

    test "struct type specs are accurate" do
      # This test documents the type specs
      config = %Kreuzberg.ExtractionConfig{}
      {:ok, _} = Kreuzberg.ExtractionConfig.validate(config)

      metadata = %Kreuzberg.Metadata{}
      assert is_struct(metadata)

      table = %Kreuzberg.Table{}
      assert is_struct(table)

      chunk = Kreuzberg.Chunk.new("text")
      assert is_struct(chunk)

      image = Kreuzberg.Image.new("png")
      assert is_struct(image)

      page = Kreuzberg.Page.from_map(%{"page_number" => 1, "content" => "content"})
      assert is_struct(page, Kreuzberg.Page)
    end
  end
end
