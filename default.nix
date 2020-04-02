let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
in
with nixpkgs;
stdenv.mkDerivation {
    name = "radicle-surf-dev";
    buildInputs = [
        (nixpkgs.rustChannelOf { rustToolChain = ./rust-toolchain; }).rust
        pkgconfig
        zlib
        openssl
        rustup
    ];
}
