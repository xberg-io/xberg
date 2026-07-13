require "./spec_helper"

describe Xberg do
  describe "batch" do
    it "extract_batch: happy path with mixed inputs" do
      __result = Xberg.extract_batch(Array(Xberg::ExtractInput).from_json("[{\"bytes\":[72,101,108,108,111,44,32,119,111,114,108,100,33],\"kind\":\"bytes\",\"mime_type\":\"text/plain\"},{\"bytes\":[60,104,116,109,108,62,60,98,111,100,121,62,84,101,115,116,60,47,98,111,100,121,62,60,47,104,116,109,108,62],\"kind\":\"bytes\",\"mime_type\":\"text/html\"}]"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
      __result.try(&.results).to_s.size.should be >=(1)
    end
    it "extract_batch with invalid bytes MIME type" do
      __result = Xberg.extract_batch(Array(Xberg::ExtractInput).from_json("[{\"bytes\":[72,101,108,108,111],\"kind\":\"bytes\",\"mime_type\":\"application/x-nonexistent\"}]"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
    end
    it "extract_batch: handles unsupported MIME gracefully" do
      __result = Xberg.extract_batch(Array(Xberg::ExtractInput).from_json("[{\"bytes\":[80,68,70,32,112,108,97,99,101,104,111,108,100,101,114],\"kind\":\"bytes\",\"mime_type\":\"application/x-unknown\"}]"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
    end
    it "extract_batch: archive size cap triggers error" do
      expect_raises(Exception) do
        Xberg.extract_batch(Array(Xberg::ExtractInput).from_json("[{\"bytes\":[97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97,97],\"kind\":\"bytes\",\"mime_type\":\"text/plain\"}]"), Xberg::ExtractionConfig.from_json("{\"security_limits\":{\"max_content_size\":1}}"))
      end
    end
    it "extract_batch with unsupported bytes MIME type" do
      __result = Xberg.extract_batch(Array(Xberg::ExtractInput).from_json("[{\"bytes\":[100,97,116,97],\"kind\":\"bytes\",\"mime_type\":\"application/x-unknown\"}]"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
    end
    it "extract_batch: empty batch" do
      __result = Xberg.extract_batch(Array(Xberg::ExtractInput).from_json("[]"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
      __result.try(&.results).to_s.size.should eq(0)
    end
    it "extract_batch with missing URI inputs" do
      __result = Xberg.extract_batch(Array(Xberg::ExtractInput).from_json("[{\"kind\":\"uri\",\"uri\":\"/nonexistent/a.pdf\"},{\"kind\":\"uri\",\"uri\":\"/nonexistent/b.txt\"}]"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
    end
    it "extract_batch over URI inputs" do
      __result = Xberg.extract_batch(Array(Xberg::ExtractInput).from_json("[{\"kind\":\"uri\",\"uri\":\"pdf/fake_memo.pdf\"},{\"kind\":\"uri\",\"uri\":\"text/fake_text.txt\"}]"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
    end
    it "extract_batch with missing URI input" do
      __result = Xberg.extract_batch(Array(Xberg::ExtractInput).from_json("[{\"kind\":\"uri\",\"uri\":\"/nonexistent/a.pdf\"}]"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
    end
    it "extract_batch with mixed valid and missing URI inputs" do
      __result = Xberg.extract_batch(Array(Xberg::ExtractInput).from_json("[{\"kind\":\"uri\",\"uri\":\"text/plain.txt\"},{\"kind\":\"uri\",\"uri\":\"/nonexistent/missing.pdf\"}]"), Xberg::ExtractionConfig.from_json("null"))
      # TODO: unsupported assertion `not_error`
    end
  end
end
