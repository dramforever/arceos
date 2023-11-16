{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = { self, nixpkgs, rust-overlay, ... }: {
    devShells = nixpkgs.lib.genAttrs [ "x86_64-linux" ] (system: {
      default =
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
        in pkgs.mkShell.override { stdenv = pkgs.stdenvNoCC; } {
          buildInputs = [
            pkgs.cargo-binutils
            (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
            # (pkgs.rust-bin.selectLatestNightlyWith
            #   (toolchain: toolchain.default.override {
            #     extensions = [ "rust-src" "llvm-tools-preview" "rustfmt" "clippy" ];
            #     targets = [ "riscv64gc-unknown-none-elf" ];
            #   }))
          ];

          shellHook = ''
            unset OBJCOPY
            export LIBCLANG_PATH=${pkgs.lib.makeLibraryPath [ pkgs.libclang ]}
          '';
        };
    });
  };
}
