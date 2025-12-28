# Advanced keyword extraction configuration
alias Kreuzberg.ExtractionConfig

config = %ExtractionConfig{
  keyword_extraction: %{
    "enabled" => true,
    "max_keywords" => 20,
    "min_score" => 0.6,
    "algorithm" => "tfidf"
  }
}

{:ok, result} = Kreuzberg.extract_file("research_paper.pdf", nil, config)

if result.keywords do
  # Group by score ranges
  high_score = Enum.filter(result.keywords, fn kw -> kw["score"] >= 0.8 end)
  medium_score = Enum.filter(result.keywords, fn kw -> kw["score"] >= 0.6 and kw["score"] < 0.8 end)

  IO.puts("High confidence keywords (#{length(high_score)}):")
  Enum.each(high_score, fn kw -> IO.puts("  - #{kw["word"]} (#{kw["score"]})") end)
end
