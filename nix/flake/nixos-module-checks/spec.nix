args@{
  pkgs,
  nixosModuleTest,
  zramTuningOnlyModuleTest,
  ...
}:

let
  json = import ./spec/json.nix args;
  activation = import ./spec/activation.nix args;
  steadyState = import ./spec/steady-state.nix args;
  handoffZram = import ./spec/handoff-zram.nix args;
in
{
  nixosModuleSpec = pkgs.runCommand "disk-nix-nixos-module-spec-check" { } ''
    test -e ${json}
    test -e ${activation}
    test -e ${steadyState}
    test -e ${handoffZram}
    touch "$out"
  '';
}
