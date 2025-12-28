```elixir title="Elixir"
# Extract images from documents for further processing
# Images are returned as base64-encoded data with format and size information

alias Kreuzberg.ExtractionConfig

defmodule ImageExtractor do
  @moduledoc """
  Extract and process images from documents.
  Provides utilities for working with extracted image data.
  """

  @doc """
  Extract all images from a document.

  Returns a list of image data with format and metadata.
  """
  def extract_images(file_path) do
    config = %ExtractionConfig{
      # Enable image extraction
      images: %{
        "extract" => true
      },
      use_cache: true
    }

    case Kreuzberg.extract_file(file_path, nil, config) do
      {:ok, result} ->
        {:ok, result.images || []}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Save extracted images to disk.

  Takes extracted images and writes them to individual files.
  """
  def save_images(images, output_dir) do
    File.mkdir_p!(output_dir)

    Enum.with_index(images, 1) |> Enum.map(fn {image, idx} ->
      format = image["format"] || "png"
      filename = "#{output_dir}/image_#{idx}.#{format}"

      # Decode base64 image data
      case Base.decode64(image["data"]) do
        {:ok, binary_data} ->
          File.write!(filename, binary_data)
          {:ok, filename}

        :error ->
          {:error, "Failed to decode image #{idx}"}
      end
    end)
  end

  @doc """
  Get image statistics from extracted images.
  """
  def get_image_stats(images) do
    {count, total_size, formats} = Enum.reduce(images, {0, 0, %{}}, fn image, {count, size, formats} ->
      new_count = count + 1
      new_size = size + (image["size"] || 0)
      format = image["format"] || "unknown"
      new_formats = Map.update(formats, format, 1, &(&1 + 1))

      {new_count, new_size, new_formats}
    end)

    %{
      total_images: count,
      total_bytes: total_size,
      formats: formats,
      avg_size: if(count > 0, do: div(total_size, count), else: 0)
    }
  end
end

# Usage example
file_path = "document_with_images.pdf"

IO.puts("Extracting images from: #{file_path}\n")

case ImageExtractor.extract_images(file_path) do
  {:ok, images} ->
    IO.puts("Found #{length(images)} image(s)\n")

    # Get image statistics
    stats = ImageExtractor.get_image_stats(images)
    IO.puts("=== Image Statistics ===")
    IO.puts("Total images: #{stats.total_images}")
    IO.puts("Total size: #{stats.total_bytes} bytes (#{div(stats.total_bytes, 1024)} KB)")
    IO.puts("Average size: #{stats.avg_size} bytes")
    IO.puts("Formats: #{inspect(stats.formats)}")
    IO.puts("")

    # Display individual image information
    IO.puts("=== Individual Images ===")
    Enum.with_index(images, 1) |> Enum.each(fn {image, idx} ->
      IO.puts("Image #{idx}:")
      IO.puts("  Format: #{image["format"]}")
      IO.puts("  Size: #{image["size"]} bytes")

      # Optional: show dimensions if available
      if image["width"] && image["height"] do
        IO.puts("  Dimensions: #{image["width"]}x#{image["height"]} pixels")
      end

      # Optional: show DPI if available
      if image["dpi"] do
        IO.puts("  DPI: #{image["dpi"]}")
      end

      IO.puts("")
    end)

    # Save images to disk
    case ImageExtractor.save_images(images, "/tmp/extracted_images") do
      results ->
        successful = Enum.count(results, fn
          {:ok, _path} -> true
          _ -> false
        end)
        IO.puts("Saved #{successful}/#{length(results)} images to /tmp/extracted_images")
    end

  {:error, reason} ->
    IO.puts("Error extracting images: #{reason}")
end
```
