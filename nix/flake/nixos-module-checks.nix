args@{
  format,
  nixosModuleTest,
  ...
}:

{
  formatting = format.check;
  nixosModule = nixosModuleTest.config.system.build.toplevel;
}
// import ./nixos-module-checks/spec.nix args
// import ./nixos-module-checks/modes.nix args
// import ./nixos-module-checks/assertions.nix args
