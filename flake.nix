{
  description = "Kreuzberg, document extraction library";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        coreShell = with pkgs; mkShell {
          nativeBuildInputs = [
            rust
            pkg-config
            git
            cmake
            gnumake
            curl
            go-task

            # Linting tools
            taplo
            shfmt
            shellcheck
            actionlint
            cppcheck
          ];

          buildInputs = [
            tesseract
            file
          ] ++ lib.optionals stdenv.isDarwin [
            apple-sdk_15
          ];

          env = {
            TESSDATA_PREFIX = "${tesseract}/share/tessdata";
          };

          shellHook = ''
            echo "kreuzberg dev shell ready ($(rustc --version))"
          '';
        };

        extendCore = extra: pkgs.mkShell {
          inputsFrom = [ coreShell ];
          nativeBuildInputs = extra;
        };
      in
      {
        devShells = {
          default = coreShell;

          python = extendCore (with pkgs; [ python3 uv ]);
          node = extendCore (with pkgs; [ nodejs corepack ]);
          go = extendCore (with pkgs; [ go ]);
          ruby = extendCore (with pkgs; [ ruby ]);
          elixir = extendCore (with pkgs; [ elixir erlang ]);
          dotnet = extendCore (with pkgs; [ dotnet-sdk ]);
          r = extendCore (with pkgs; [ R ]);
          php = extendCore (with pkgs; [ php ]);
          wasm = extendCore (with pkgs; [ deno wasm-pack ]);

          full = extendCore (with pkgs; [
            python3
            uv
            nodejs
            corepack
            go
            ruby
            elixir
            erlang
            dotnet-sdk
            R
            php
            deno
            wasm-pack
          ]);
        };
      }
    );
}
