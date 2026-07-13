require "./spec_helper"

describe Xberg do
  describe "url" do
    it "extract_batch: mixed bytes and URL inputs share one output envelope" do
      __result = Xberg.extract_batch(Array(Xberg::ExtractInput).from_json("[{\"kind\":\"uri\",\"uri\":\"$mock_url\"},{\"bytes\":[66,97,116,99,104,32,98,121,116,101,115,32,99,111,110,116,101,110,116],\"filename\":\"inline.txt\",\"kind\":\"bytes\",\"mime_type\":\"text/plain\"}]"), Xberg::ExtractionConfig.from_json("{\"url\":{\"mode\":\"document\"}}"))
      # TODO: unsupported assertion `not_error`
      __result.try(&.results).to_s.size.should be >=(2)
      __result.results[0].content.to_s.should contain("Batch URL document content")
      __result.results[1].content.to_s.should contain("Batch bytes content")
    end
    it "extract: crawl mode follows linked pages" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("{\"kind\":\"uri\",\"uri\":\"$mock_url\"}"), Xberg::ExtractionConfig.from_json("{\"url\":{\"crawl\":{\"max_depth\":1,\"max_pages\":4,\"respect_robots_txt\":false},\"mode\":\"crawl\"}}"))
      # TODO: unsupported assertion `not_error`
      (__result.try(&.summary).try(&.pages_crawled) || 0).should be >= 2
      __result.results[1].content.to_s.should contain("About crawl target")
    end
    it "extract: website URL returns page content" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("{\"kind\":\"uri\",\"uri\":\"$mock_url\"}"), Xberg::ExtractionConfig.from_json("{\"url\":{\"mode\":\"document\"}}"))
      # TODO: unsupported assertion `not_error`
      __result.results[0].content.to_s.should contain("Xberg URL Page")
      __result.try(&.results).to_s.size.should be >=(1)
    end
    it "extract: recursive URL extraction follows document links discovered in results" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("{\"kind\":\"uri\",\"uri\":\"$mock_url\"}"), Xberg::ExtractionConfig.from_json("{\"url\":{\"crawl\":{\"document_url_depth\":1,\"follow_document_urls\":true,\"respect_robots_txt\":false},\"mode\":\"document\"}}"))
      # TODO: unsupported assertion `not_error`
      __result.try(&.results).to_s.size.should be >=(2)
      __result.results[1].content.to_s.should contain("Recursive document target")
    end
    it "extract: remote text document URL" do
      __result = Xberg.extract(Xberg::ExtractInput.from_json("{\"kind\":\"uri\",\"uri\":\"$mock_url\"}"), Xberg::ExtractionConfig.from_json("{\"url\":{\"mode\":\"document\"}}"))
      # TODO: unsupported assertion `not_error`
      __result.results[0].content.to_s.should contain("Remote document hello")
      __result.try(&.summary).try(&.remote_urls).should eq(1)
    end
  end
end
