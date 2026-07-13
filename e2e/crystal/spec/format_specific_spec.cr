require "./spec_helper"

describe Xberg do
  describe "format_specific" do
    it "Standalone DOCX extraction using extract" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
      __result.results[0].content.size.should be >=(20)
    end
    it "Standalone HWPX extraction using extract" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
      __result.results[0].content.size.should be >=(20)
      __result.results[0].content.to_s.should contain("Hello from HWPX")
    end
    it "Standalone PDF text extraction using extract" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
      __result.results[0].content.size.should be >=(50)
      (__result.results[0].content.includes?("Mallori") || __result.results[0].content.includes?("May")).should be_true
    end
    it "PPTX presentation extraction using extract" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
    end
    it "XLSX spreadsheet extraction using extract" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
    end
  end
end
