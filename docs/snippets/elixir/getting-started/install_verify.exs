```elixir title="Elixir"
# Verify Xberg is installed and working by extracting a document
{:ok, result} = Xberg.extract("sample.pdf")
IO.puts("Installation verified! Extracted #{String.length(result.content)} characters")
```
