{ sources ? import ./nix/sources.nix
, pkgs ? import sources.nixpkgs {
    overlays = [ (import sources.rust-overlay) ];
  }
}:
let
  stable = pkgs.rust-bin.stable.latest.default;
  rust = stable.override {
    extensions = [ "rust-src" "rust-analysis" ];
  };
in
with pkgs;
mkShell {
    name = "radicle-surf";
    buildInputs = [
        clang
        cargo-deny
        cargo-expand
        cargo-watch
        # gnuplot for benchmark purposes
        gnuplot
        lld
        pkgconfig
        openssl
        pkgs.rust-bin.nightly."2021-12-02".rustfmt
        ripgrep
        rust
        zlib
    ];
}
