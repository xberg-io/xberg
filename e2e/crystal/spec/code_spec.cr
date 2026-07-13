require "./spec_helper"

describe Xberg do
  describe "code" do
    it "Test language detection from shebang line via bytes input" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("null"))
      __result.results[0].mime_type.should eq("text/x-source-code")
      __result.results[0].content.size.should be >=(10)
      __result.results[0].content.to_s.should contain("build")
      __result.results[0].content.to_s.should contain("clean")
    end
  end
end
