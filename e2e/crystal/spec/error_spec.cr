require "./spec_helper"

describe Xberg do
  describe "error" do
    it "Graceful handling of empty bytes (should not error)" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      # TODO: unsupported assertion `not_error`
    end
    it "Error when extracting with empty MIME type" do
      expect_raises(Exception) do
        Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      end
    end
    it "extract force+disable OCR" do
      expect_raises(Exception) do
        Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{\"disable_ocr\":true,\"force_ocr\":true}"))
      end
    end
    it "Error when extracting with invalid MIME type format" do
      expect_raises(Exception) do
        Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      end
    end
    it "Error when extracting with unsupported MIME type" do
      expect_raises(Exception) do
        Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      end
    end
  end
end
