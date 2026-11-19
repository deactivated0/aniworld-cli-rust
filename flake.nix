{
  description = "Rust development shell for aniworld-cli-rust";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forEachSystem = f: nixpkgs.lib.genAttrs systems (system: f system);
    in {
      devShells = forEachSystem (system: {
        default = let pkgs = nixpkgs.legacyPackages.${system}; in pkgs.mkShell {
          packages = with pkgs; [ rustup mpv pkg-config ];
          shellHook = ''
            echo "aniworld-cli-rust: cargo build --release"
          '';
        };
      });
    };
}
