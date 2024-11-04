{ pkgs ? import <nixpkgs> {} }:
  pkgs.mkShell {
    # nativeBuildInputs is usually what you want -- tools you need to run
    nativeBuildInputs = with pkgs.buildPackages; [ neovim git which openssh cargo rustc clippy openssl_3_3 pkg-config ];
    shellHook = ''
        alias g=git
        alias n="nvim ."
        source .env
    '';
}
