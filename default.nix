with import <nixpkgs> {};
stdenv.mkDerivation {
    name = "radicle-surf-dev";
    buildInputs = [ pkgconfig zlib openssl rustup ];
}
