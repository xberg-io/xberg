```elixir title="Elixir"
config = Jason.encode!(%{})

case Xberg.extract_sync("document.pdf", nil, config) do
  {:ok, result} ->
    with %{"metadata" => %{"pdf" => pdf_meta}} <- Jason.decode!(result) do
      case pdf_meta do
        %{"page_count" => pages} ->
          IO.puts("Pages: #{pages}")
        _ ->
          nil
      end

      case pdf_meta do
        %{"author" => author} ->
          IO.puts("Author: #{author}")
        _ ->
          nil
      end

      case pdf_meta do
        %{"title" => title} ->
          IO.puts("Title: #{title}")
        _ ->
          nil
      end
    end

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end

case Xberg.extract_sync("page.html", nil, config) do
  {:ok, result} ->
    with %{"metadata" => %{"html" => html_meta}} <- Jason.decode!(result) do
      case html_meta do
        %{"title" => title} ->
          IO.puts("Title: #{title}")
        _ ->
          nil
      end

      case html_meta do
        %{"description" => desc} ->
          IO.puts("Description: #{desc}")
        _ ->
          nil
      end

      # Access keywords array
      case html_meta do
        %{"keywords" => keywords} ->
          IO.inspect(keywords, label: "Keywords")
        _ ->
          nil
      end

      # Access canonical URL
      case html_meta do
        %{"canonical_url" => canonical} ->
          IO.puts("Canonical URL: #{canonical}")
        _ ->
          nil
      end

      # Access Open Graph fields as a map
      case html_meta do
        %{"open_graph" => og} when is_map(og) ->
          case og do
            %{"image" => og_image} ->
              IO.puts("Open Graph Image: #{og_image}")
            _ ->
              nil
          end

          case og do
            %{"title" => og_title} ->
              IO.puts("Open Graph Title: #{og_title}")
            _ ->
              nil
          end

        _ ->
          nil
      end

      # Access Twitter Card fields as a map
      case html_meta do
        %{"twitter_card" => tc} when is_map(tc) ->
          case tc do
            %{"card" => card_type} ->
              IO.puts("Twitter Card Type: #{card_type}")
            _ ->
              nil
          end

        _ ->
          nil
      end

      # Access language
      case html_meta do
        %{"language" => lang} ->
          IO.puts("Language: #{lang}")
        _ ->
          nil
      end

      # Access headers
      case html_meta do
        %{"headers" => headers} when is_list(headers) and length(headers) > 0 ->
          Enum.each(headers, fn header ->
            IO.puts("Header (level #{header["level"]}): #{header["text"]}")
          end)

        _ ->
          nil
      end

      # Access links
      case html_meta do
        %{"links" => links} when is_list(links) and length(links) > 0 ->
          Enum.each(links, fn link ->
            IO.puts("Link: #{link["href"]} (#{link["text"]})")
          end)

        _ ->
          nil
      end

      # Access images
      case html_meta do
        %{"images" => images} when is_list(images) and length(images) > 0 ->
          Enum.each(images, fn image ->
            IO.puts("Image: #{image["src"]}")
          end)

        _ ->
          nil
      end

      # Access structured data
      case html_meta do
        %{"structured_data" => sd} when is_list(sd) ->
          IO.puts("Structured data items: #{length(sd)}")
        _ ->
          nil
      end
    end

  {:error, reason} ->
    IO.puts("Error: #{reason}")
end
```
