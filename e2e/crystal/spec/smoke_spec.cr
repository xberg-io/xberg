require "./spec_helper"

describe Xberg do
  describe "smoke" do
    it "OCR: PNG image extraction with OCR enabled. In WASM this exercises the Uint8Array bridge parameter and Promise await in the generated OcrBackend bridge." do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      __result.results[0].mime_type.should eq("image/png")
      __result.results[0].content.size.should be >=(1)
      (__result.results[0].content.includes?("Hello") || __result.results[0].content.includes?("World") || __result.results[0].content.includes?("hello") || __result.results[0].content.includes?("world")).should be_true
    end
    it "Smoke test: DOCX with formatted text" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      __result.results[0].mime_type.should eq("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
      __result.results[0].content.size.should be >=(20)
      (__result.results[0].content.includes?("Lorem") || __result.results[0].content.includes?("ipsum") || __result.results[0].content.includes?("document") || __result.results[0].content.includes?("text")).should be_true
    end
    it "Smoke test: HTML table extraction" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      __result.results[0].mime_type.should eq("text/html")
      __result.results[0].content.size.should be >=(10)
      (__result.results[0].content.includes?("Sample Data Table") || __result.results[0].content.includes?("Laptop") || __result.results[0].content.includes?("Electronics") || __result.results[0].content.includes?("Product")).should be_true
    end
    it "Smoke test: PNG image (without OCR, metadata only)" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{\"disable_ocr\":true}"))
      __result.results[0].mime_type.should eq("image/png")
    end
    it "Smoke test: JSON file extraction" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      __result.results[0].mime_type.should eq("application/json")
      __result.results[0].content.size.should be >=(5)
    end
    it "Smoke test: PDF with simple text extraction" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      __result.results[0].mime_type.should eq("application/pdf")
      __result.results[0].content.size.should be >=(50)
      (__result.results[0].content.includes?("May 5, 2023") || __result.results[0].content.includes?("To Whom it May Concern")).should be_true
    end
    it "Smoke test: Plain text file" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      __result.results[0].mime_type.should eq("text/plain")
      __result.results[0].content.size.should be >=(5)
    end
    it "Smoke test: XLSX with basic spreadsheet data including tables" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("null"), Xberg::ExtractionConfig.from_json("{}"))
      __result.results[0].mime_type.should eq("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
      __result.results[0].content.size.should be >=(100)
      __result.results[0].content.to_s.should contain("Team")
      __result.results[0].content.to_s.should contain("Location")
      __result.results[0].content.to_s.should contain("Stanley Cups")
      __result.results[0].content.to_s.should contain("Blues")
      __result.results[0].content.to_s.should contain("Flyers")
      __result.results[0].content.to_s.should contain("Maple Leafs")
      __result.results[0].content.to_s.should contain("STL")
      __result.results[0].content.to_s.should contain("PHI")
      __result.results[0].content.to_s.should contain("TOR")
      __result.results[0].tables.size.should be >=(1)
      (__result.results[0].try(&.metadata).try(&.format).as?(Xberg::FormatMetadata::Excel).try(&.sheet_count) || 0).should be >= 2
      __result.results[0].try(&.metadata).try(&.format).as?(Xberg::FormatMetadata::Excel).try(&.sheet_names).to_s.should contain("Stanley Cups")
    end
  end
end
