{
  pkgs,
  formatProgram,
}:

{
  default = pkgs.mkShell {
    packages = [
      pkgs.cargo
      pkgs.clippy
      pkgs.rustc
      pkgs.rustfmt
      pkgs.rust-analyzer
      pkgs.pkg-config
      pkgs.just
      formatProgram
    ];
  };
}
