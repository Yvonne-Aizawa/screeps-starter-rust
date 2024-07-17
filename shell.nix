let
  rust_overlay = import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
in
with pkgs;
mkShell {
  buildInputs = [
    gdb
    picocom
    udev pkg-config
    openssl
    sqlitecpp
    sqlite
    (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
  ];
  RUST_LOG = "INFO";
}