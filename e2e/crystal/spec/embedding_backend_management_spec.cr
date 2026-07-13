require "./spec_helper"

describe Xberg do
  describe "embedding_backend_management" do
    it "List all registered embedding backends" do
      __result = Xberg.list_embedding_backends()
      # TODO: unsupported assertion `not_error`
    end
  end
end
