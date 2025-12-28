defmodule Kreuzberg.Plugin.Supervisor do
  @moduledoc """
  OTP Supervisor for the Kreuzberg plugin system.

  This supervisor manages the plugin system's core components, specifically
  the Registry GenServer that maintains the registry of loaded plugins.

  ## Supervision Strategy

  Uses a `:one_for_one` strategy, meaning if the Registry process terminates,
  only that process will be restarted, not the entire supervision tree.

  ## Usage

  The supervisor is typically started automatically as part of the Kreuzberg
  application supervision tree. To start it manually:

      {:ok, pid} = Kreuzberg.Plugin.Supervisor.start_link([])

  ## Children

  - `Kreuzberg.Plugin.Registry` - GenServer managing plugin registration and lookup
  """

  use Supervisor

  @doc """
  Start the plugin supervisor.

  ## Options
    * `:name` - Registered process name (defaults to module name)

  ## Returns
    * `{:ok, pid()}` - Successfully started
    * `{:error, reason}` - Failed to start
  """
  @spec start_link(keyword()) :: Supervisor.on_start()
  def start_link(opts) do
    Supervisor.start_link(__MODULE__, opts, name: Keyword.get(opts, :name, __MODULE__))
  end

  @doc """
  Return the supervisor child specification.

  This is useful for including the plugin supervisor in a parent supervision tree.

  ## Returns
    * Child specification map
  """
  @spec child_spec(keyword()) :: Supervisor.child_spec()
  def child_spec(opts) do
    %{
      id: __MODULE__,
      start: {__MODULE__, :start_link, [opts]},
      type: :supervisor
    }
  end

  @impl true
  def init(_opts) do
    children = [
      {Kreuzberg.Plugin.Registry, [name: Kreuzberg.Plugin.Registry]}
    ]

    Supervisor.init(children, strategy: :one_for_one)
  end
end
