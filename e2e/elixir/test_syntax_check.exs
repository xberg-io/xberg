unless Code.ensure_loaded?(E2e.TestStubs.TestStubRegisterDocumentExtractorTraitBridge) do
defmodule E2e.TestStubs.TestStubRegisterDocumentExtractorTraitBridge do
  def name, do: "test-extractor"
  def initialize, do: :ok
  def extract(input, config), do: {:ok, %{}}
  def supported_mime_types, do: []
end
end

unless Code.ensure_loaded?(E2e.TestStubs.TestStubRegisterDocumentExtractorTraitBridgeGenServer) do
defmodule E2e.TestStubs.TestStubRegisterDocumentExtractorTraitBridgeGenServer do
  use GenServer

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, nil)
  end

  @impl true
  def init(_), do: {:ok, nil}

  @impl true
  def handle_info({:trait_call, method_atom, args_json, reply_id}, state) do
    args = Jason.decode!(args_json)
    result = apply(E2e.TestStubs.TestStubRegisterDocumentExtractorTraitBridge, method_atom, args)
    result_json = Jason.encode!(result)
    Xberg.Native.complete_trait_call(reply_id, result_json)
    {:noreply, state}
  end
end
end

defmodule E2e.PluginApiTest do
  use ExUnit.Case

  describe "register_document_extractor_trait_bridge" do
    test "register_document_extractor_trait_bridge" do
      {:ok, registerdocumentextractortraitbridge_pid} = E2e.TestStubs.TestStubRegisterDocumentExtractorTraitBridgeGenServer.start_link(nil)
      result = Xberg.register_document_extractor(registerdocumentextractortraitbridge_pid, "test-extractor")
    end
  end
end
