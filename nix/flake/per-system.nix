{
  self,
  nixpkgs,
  system,
  root,
}:

let
  pkgs = import nixpkgs { inherit system; };
  format = import ./formatter.nix { inherit pkgs self; };
  formatProgram = format.formatter;
  diskNix = import ./package.nix { inherit pkgs self; };

  integrations = import ./integrations.nix { inherit pkgs root diskNix; };
  inherit (integrations)
    integrationLoopSmoke
    integrationBtrfsSmoke
    integrationBcachefsSmoke
    integrationBcacheSmoke
    integrationLuksSmoke
    integrationSwapSmoke
    integrationZramSmoke
    integrationLvmSmoke
    integrationMdraidSmoke
    integrationZfsSmoke
    integrationNfsSmoke
    integrationVdoSmoke
    integrationIscsiSmoke
    integrationMultipathSmoke
    integrationNvmeSmoke
    integrationTargetLunSmoke
    integrationFailureRecoverySmoke
    integrationLayeredVmSmoke
    integrationDiskoExamples
    integrationVmSmoke
    integrationVmTest
    integrationDiskoKernelExamplesVmTest
    ;
  integrationArgs = {
    inherit
      integrationLoopSmoke
      integrationBtrfsSmoke
      integrationBcachefsSmoke
      integrationBcacheSmoke
      integrationLuksSmoke
      integrationSwapSmoke
      integrationZramSmoke
      integrationLvmSmoke
      integrationMdraidSmoke
      integrationZfsSmoke
      integrationNfsSmoke
      integrationVdoSmoke
      integrationIscsiSmoke
      integrationMultipathSmoke
      integrationNvmeSmoke
      integrationTargetLunSmoke
      integrationFailureRecoverySmoke
      integrationLayeredVmSmoke
      integrationDiskoExamples
      integrationVmSmoke
      integrationVmTest
      integrationDiskoKernelExamplesVmTest
      ;
  };

  nixosModuleTests = import ./nixos-module-tests.nix { inherit pkgs self; };
  inherit (nixosModuleTests)
    nixosModuleTest
    zramTuningOnlyModuleTest
    nixosModuleExecuteTest
    nixosModuleHandoffAutoImportTest
    nixosModuleBootModeTest
    nixosModuleInstallModeTest
    nixosModuleCollisionTest
    nixosModuleDiskCollisionTest
    nixosModulePartitionCollisionTest
    nixosModuleLuksKeyslotCollisionTest
    nixosModuleLuksTokenCollisionTest
    nixosModuleBackingFileCollisionTest
    nixosModuleBtrfsSubvolumeCollisionTest
    nixosModuleBtrfsQgroupCollisionTest
    nixosModuleDmMapCollisionTest
    nixosModuleVdoVolumeCollisionTest
    nixosModulePhysicalVolumeCollisionTest
    nixosModuleLoopDeviceCollisionTest
    nixosModuleMdRaidCollisionTest
    nixosModuleMultipathMapCollisionTest
    nixosModuleNvmeNamespaceCollisionTest
    nixosModuleCacheCollisionTest
    nixosModulePoolCollisionTest
    nixosModuleDatasetCollisionTest
    nixosModuleZvolCollisionTest
    nixosModuleVolumeGroupCollisionTest
    nixosModuleVolumeCollisionTest
    nixosModuleThinPoolCollisionTest
    nixosModuleLvmCacheCollisionTest
    nixosModuleSnapshotCollisionTest
    nixosModuleIscsiSessionCollisionTest
    nixosModuleLunPathCollisionTest
    ;
  nixosModuleCheckArgs = {
    inherit
      nixosModuleTest
      zramTuningOnlyModuleTest
      nixosModuleExecuteTest
      nixosModuleHandoffAutoImportTest
      nixosModuleBootModeTest
      nixosModuleInstallModeTest
      nixosModuleCollisionTest
      nixosModuleDiskCollisionTest
      nixosModulePartitionCollisionTest
      nixosModuleLuksKeyslotCollisionTest
      nixosModuleLuksTokenCollisionTest
      nixosModuleBackingFileCollisionTest
      nixosModuleBtrfsSubvolumeCollisionTest
      nixosModuleBtrfsQgroupCollisionTest
      nixosModuleDmMapCollisionTest
      nixosModuleVdoVolumeCollisionTest
      nixosModulePhysicalVolumeCollisionTest
      nixosModuleLoopDeviceCollisionTest
      nixosModuleMdRaidCollisionTest
      nixosModuleMultipathMapCollisionTest
      nixosModuleNvmeNamespaceCollisionTest
      nixosModuleCacheCollisionTest
      nixosModulePoolCollisionTest
      nixosModuleDatasetCollisionTest
      nixosModuleZvolCollisionTest
      nixosModuleVolumeGroupCollisionTest
      nixosModuleVolumeCollisionTest
      nixosModuleThinPoolCollisionTest
      nixosModuleLvmCacheCollisionTest
      nixosModuleSnapshotCollisionTest
      nixosModuleIscsiSessionCollisionTest
      nixosModuleLunPathCollisionTest
      ;
  };

  packageArgs = {
    inherit diskNix;
  }
  // integrationArgs;
  checkArgs = {
    inherit
      pkgs
      self
      root
      diskNix
      format
      ;
  }
  // integrationArgs
  // nixosModuleCheckArgs;
in
{
  formatter = formatProgram;
  packages = import ./packages.nix packageArgs;
  apps = import ./apps.nix packageArgs;
  checks = import ./checks.nix checkArgs;
  devShells = import ./dev-shells.nix { inherit pkgs formatProgram; };
}
