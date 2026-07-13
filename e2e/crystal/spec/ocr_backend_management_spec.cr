require "./spec_helper"

describe Xberg do
  describe "ocr_backend_management" do
    it "List all registered OCR backends" do
      __result = Xberg.list_ocr_backends()
      # TODO: unsupported assertion `not_error`
    end
  end
end
