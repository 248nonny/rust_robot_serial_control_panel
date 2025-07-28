{ pkgs ? import <nixpkgs> {} }:

let
  llvmPkg = pkgs.llvmPackages_19;  # You can choose llvmPackages_X if needed
in

with pkgs;
mkShell {
  nativeBuildInputs = with xorg; [

    libxcb
    libXcursor
    libXrandr
    libXi
    pkg-config
    libxkbcommon
  ] ++ [
    libGL
    libGLU
    llvmPkg.clang
    llvmPkg.libclang
  ];

  buildInputs = with pkgs; [
    cargo
    rustc
    systemd
    xorg.libX11
    wayland
    libxkbcommon
  ];
  
  LIBCLANG_PATH = "${llvmPkg.libclang.lib}/lib";  # <-- this is critical
  # Optional: if using bindgen with C++ headers
  BINDGEN_EXTRA_CLANG_ARGS = "-I${llvmPkg.clang.libc}/lib/clang/${llvmPkg.clang.version}/include";

  shellHook = ''
      export LD_LIBRARY_PATH=/run/opengl-driver/lib/:${lib.makeLibraryPath ([libGL libGLU libxkbcommon])}
  '';
  
}
