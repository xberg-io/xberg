# Token reduction for different LLM context windows
alias Kreuzberg.ExtractionConfig

# For GPT-4
config_gpt4 = %ExtractionConfig{
  token_reduction: %{
    "enabled" => true,
    "target_tokens" => 8000,
    "strategy" => "intelligent",
    "preserve_structure" => true
  }
}

# For Claude
config_claude = %ExtractionConfig{
  token_reduction: %{
    "enabled" => true,
    "target_tokens" => 100000,
    "strategy" => "minimal"
  }
}
