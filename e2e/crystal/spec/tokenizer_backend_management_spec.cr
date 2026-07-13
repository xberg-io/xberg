require "./spec_helper"

describe Xberg do
  describe "tokenizer_backend_management" do
    it "List all registered tokenizer backends" do
      __result = Xberg.list_tokenizer_backends()
      # TODO: unsupported assertion `not_error`
    end
  end
end
