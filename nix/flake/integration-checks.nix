args@{
  pkgs,
  root,
  diskNix,
  integrationLoopSmoke,
  integrationBtrfsSmoke,
  integrationBcachefsSmoke,
  integrationBcacheSmoke,
  integrationLuksSmoke,
  integrationSwapSmoke,
  integrationZramSmoke,
  integrationLvmSmoke,
  integrationMdraidSmoke,
  integrationZfsSmoke,
  integrationNfsSmoke,
  integrationVdoSmoke,
  integrationIscsiSmoke,
  integrationMultipathSmoke,
  integrationNvmeSmoke,
  integrationTargetLunSmoke,
  integrationFailureRecoverySmoke,
  integrationLayeredVmSmoke,
  integrationDiskoExamples,
  integrationVmSmoke,
  ...
}:

let
  inherit (pkgs.lib) mergeAttrsList;
in
mergeAttrsList [
  (import ./integration-checks/basic-smoke.nix args)
  (import ./integration-checks/local-stack-smoke.nix args)
  (import ./integration-checks/target-lun-smoke.nix args)
  (import ./integration-checks/vm-smoke.nix args)
]
