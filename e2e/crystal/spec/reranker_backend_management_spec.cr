require "./spec_helper"

describe Xberg do
  describe "reranker_backend_management" do
    it "List all registered reranker backends" do
      __result = Xberg.list_reranker_backends()
      # TODO: unsupported assertion `not_error`
    end
  end
end
