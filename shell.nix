{ pkgs ? import <nixpkgs> {} }:

let
  llvmPkg = pkgs.llvmPackages_19;  # You can choose llvmPackages_X if needed
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
    llvmPkg.clang
    llvmPkg.libclang
  ];

  buildInputs = with pkgs; [
    systemd
  ];
  
  LIBCLANG_PATH = "${llvmPkg.libclang.lib}/lib";  # <-- this is critical
  # Optional: if using bindgen with C++ headers
  BINDGEN_EXTRA_CLANG_ARGS = "-I${llvmPkg.clang.libc}/lib/clang/${llvmPkg.clang.version}/include";
}
