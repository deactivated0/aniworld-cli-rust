{
  description = "Fast, native Rust terminal client for AniWorld anime streaming with mpv integration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "aniworld-cli-rust";
          version = "0.2.1";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ mpv ];
          meta = with pkgs.lib; {
            description = "Blazing-fast, native Rust terminal client for AniWorld";
            homepage = "https://github.com/deactivated0/aniworld-cli-rust";
            license = licenses.mit;
            mainProgram = "aniworld";
          };
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [ rustup cargo rustc mpv pkg-config ];
          shellHook = ''
            echo "aniworld-cli-rust dev shell: cargo build --release"
          '';
        };
      }
    ) // {
      overlays.default = final: prev: {
        aniworld-cli = self.packages.${final.system}.default;
      };
    };
}
