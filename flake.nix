{
  description = "Dev shell for Rust nightly with common libraries";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = { nixpkgs, rust-overlay, ... }: {
    devShells.x86_64-linux.default =                                   # Target x86_64 Linux
      let 
        pkgs = import nixpkgs {
          system = "x86_64-linux";
          overlays = [ (import rust-overlay) ];                        # Apply Rust overlay
        };
      in pkgs.mkShell {
        buildInputs = [
          # Rust nightly toolchain with full components:
          (pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
            extensions = [ "rust-src" "clippy" ];                      # Include Rust source and Clippy
            # (rustc, cargo, rustfmt are in the default profile&#8203;:contentReference[oaicite:1]{index=1})
          }))
          # Common libraries for Rust crates:
          pkgs.openssl            # OpenSSL libraries (for openssl-sys, etc.)
          pkgs.openssl.dev        # OpenSSL headers and pkg-config files&#8203;:contentReference[oaicite:2]{index=2}
          pkgs.pkg-config         # pkg-config to help find libraries&#8203;:contentReference[oaicite:3]{index=3}
          pkgs.zlib               # zlib compression library (for libz-sys)
          pkgs.sqlite             # SQLite library (for libsqlite3-sys, if needed)
          pkgs.cmake              # CMake, often needed for C/C++ build scripts
          pkgs.gcc                # GCC compiler for building C dependencies
          pkgs.lldb
          pkgs.bacon
          pkgs.rust-analyzer
          pkgs.geckodriver
        ];
      };
  };
}


