{ sources ? import ./nix/sources.nix
, pkgs ? import sources.nixpkgs {
    overlays = [ (import sources.rust-overlay) ];
  }
}:
let
    rustToolChain = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain);
    rust = rustToolChain.override {
        extensions = [ "rust-src" "rust-analysis" ];
    };
in
with pkgs;
mkShell {
    name = "radicle-surf";
    buildInputs = [
        cargo-expand
        cargo-watch
        # gnuplot for benchmark purposes
        gnuplot
        pkgconfig
        openssl
        ripgrep
        rust
        zlib
    ];
}
