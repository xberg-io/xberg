# Advanced keyword extraction configuration
alias Xberg.ExtractionConfig

config = %ExtractionConfig{
  keyword_extraction: %{
    "enabled" => true,
    "max_keywords" => 20,
    "min_score" => 0.6,
    "algorithm" => "tfidf"
  }
}

{:ok, output} = Xberg.extract(%Xberg.ExtractInput{kind: :uri, uri: "research_paper.pdf"}, config)

result = List.first(output.results)
if result.extracted_keywords do
  # Group by score ranges
  high_score = Enum.filter(result.extracted_keywords, fn kw -> kw["score"] >= 0.8 end)
  medium_score = Enum.filter(result.extracted_keywords, fn kw -> kw["score"] >= 0.6 and kw["score"] < 0.8 end)

  IO.puts("High confidence keywords (#{length(high_score)}):")
  Enum.each(high_score, fn kw -> IO.puts("  - #{kw["text"]} (#{kw["score"]})") end)
end
