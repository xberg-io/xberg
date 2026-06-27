```elixir title="Elixir"
{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}, nil)
result = List.first(output.results)
# Metadata is flat — format-specific fields are at the top level
metadata = result.metadata
IO.puts("MIME type: #{result.mime_type}")
IO.puts("All metadata keys: #{inspect(Map.keys(metadata))}")

# Access PDF metadata directly from the flat map
page_count = metadata["page_count"]
if page_count, do: IO.puts("Page count: #{page_count}")

authors = metadata["authors"] || []
if authors != [], do: IO.puts("Authors: #{Enum.join(authors, ", ")}")

title = metadata["title"]
if title, do: IO.puts("Title: #{title}")

# Access HTML metadata directly from the flat map
{:ok, html_output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "page.html"}, nil)
html_result = List.first(html_output.results)
html_meta = html_result.metadata

keywords = html_meta["keywords"] || []
if keywords != [], do: IO.puts("Keywords: #{Enum.join(keywords, ", ")}")

description = html_meta["description"]
if description, do: IO.puts("Description: #{description}")
```
