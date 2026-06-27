```elixir title="Elixir"
input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}

{:ok, output} = Xberg.extract_async(input: input)

IO.inspect(output.summary)
```
