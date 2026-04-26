{
  description = "Rusp - A typed Lisp implemented in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain
            pkgs.pkg-config
            pkgs.llvmPackages_18.llvm.dev
            pkgs.libffi
            pkgs.libxml2
            pkgs.zlib
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

          env = {
            RUST_BACKTRACE = "1";
            # llvm-sys (transitive dep of inkwell) finds llvm-config via this prefix.
            LLVM_SYS_181_PREFIX = "${pkgs.llvmPackages_18.llvm.dev}";
          };

          shellHook = ''
            echo "Rusp dev shell"
            echo "  $(rustc --version)"
            echo "  $(cargo --version)"
            echo "  $(${pkgs.llvmPackages_18.llvm.dev}/bin/llvm-config --version | head -c 32) (LLVM)"
          '';
        };
      });
}
