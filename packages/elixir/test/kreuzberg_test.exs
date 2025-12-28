defmodule KreuzbergTest do
  use ExUnit.Case
  doctest Kreuzberg

  test "module is loaded" do
    assert is_atom(Kreuzberg)
  end

  test "Native module is loaded" do
    assert is_atom(Kreuzberg.Native)
  end
end
