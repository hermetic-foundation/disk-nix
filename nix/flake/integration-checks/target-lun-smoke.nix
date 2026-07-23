{
  pkgs,
  root,
  ...
}:

let
  args = {
    inherit pkgs root;
  };
in
import ./target-lun-smoke/target-lun.nix args // import ./target-lun-smoke/failure-recovery.nix args
