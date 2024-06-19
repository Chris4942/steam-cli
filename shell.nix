{ nixpkgs ? import <nixpkgs> { }}:

let
  pinnedPkgs = nixpkgs.fetchFromGitHub {
    owner  = "NixOS";
    repo   = "nixpkgs";
    rev    = "1fe6ed37fd9beb92afe90671c0c2a662a03463dd";
    sha256 = "1daa0y3p17shn9gibr321vx8vija6bfsb5zd7h4pxdbbwjkfq8n2";
  };
  pkgs = import pinnedPkgs {};
in
  pkgs.mkShell {
    buildInputs = with pkgs; [
      clippy
      rustc
      cargo
      rustfmt
      rust-analyzer
    ];

    RUST_BACKTRACE = 1;
  }
