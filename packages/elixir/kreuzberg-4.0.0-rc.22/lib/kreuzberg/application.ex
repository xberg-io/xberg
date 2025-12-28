defmodule Kreuzberg.Application do
  @moduledoc """
  OTP Application callback for Kreuzberg.

  This module defines the application supervision tree, which starts all
  the necessary services for the Kreuzberg library.
  """

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      Kreuzberg.Plugin.Supervisor
    ]

    opts = [strategy: :one_for_one, name: Kreuzberg.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
