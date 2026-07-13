require "./spec_helper"

describe Xberg do
  describe "summarization" do
    it "LLM-driven abstractive summary. Skipped automatically when XBERG_LLM_API_KEY (or OPENAI_API_KEY) is not set." do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{\"summarization\":{\"llm\":{\"max_tokens\":200,\"model\":\"openai/gpt-4o-mini\",\"temperature\":0.0},\"max_tokens\":150,\"strategy\":\"abstractive\"}}"))
      # TODO: unsupported assertion `not_error`
      __result.results[0].try(&.summary).try(&.text).to_s.should_not be_empty
      __result.results[0].try(&.summary).try(&.strategy).should eq("abstractive")
    end
    it "TextRank extractive summary over a multi-paragraph plain text document. Pure-Rust, deterministic, no external services required." do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{\"summarization\":{\"max_tokens\":80,\"strategy\":\"extractive\"}}"))
      # TODO: unsupported assertion `not_error`
      __result.results[0].mime_type.should eq("text/plain")
      __result.results[0].try(&.summary).try(&.text).to_s.should_not be_empty
      __result.results[0].try(&.summary).try(&.strategy).should eq("extractive")
    end
  end
end
