require "./spec_helper"

describe Xberg do
  describe "extract" do
    it "extract bytes input from PDF document" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
      __result.results[0].mime_type.should eq("application/pdf")
      __result.results[0].content.size.should be >=(50)
    end
    it "extract bytes input with empty MIME type" do
      expect_raises(Exception) do
        Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      end
    end
    it "extract bytes input with unsupported MIME type" do
      expect_raises(Exception) do
        Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      end
    end
  end
end
