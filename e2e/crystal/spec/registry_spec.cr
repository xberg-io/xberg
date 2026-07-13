require "./spec_helper"

describe Xberg do
  describe "registry" do
    it "List embedding backends" do
      __result = Xberg.list_embedding_backends()
      # TODO: unsupported assertion `not_error`
    end
    it "List OCR backends" do
      __result = Xberg.list_ocr_backends()
      # TODO: unsupported assertion `not_error`
    end
    it "List post-processors" do
      __result = Xberg.list_post_processors()
      # TODO: unsupported assertion `not_error`
    end
    it "List renderers" do
      __result = Xberg.list_renderers()
      # TODO: unsupported assertion `not_error`
    end
    it "List validators" do
      __result = Xberg.list_validators()
      # TODO: unsupported assertion `not_error`
    end
  end
end
