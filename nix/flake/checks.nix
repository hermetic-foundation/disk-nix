args@{
  pkgs,
  self,
  root,
  diskNix,
  format,
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
  integrationVmTest,
  integrationDiskoKernelExamplesVmTest,
  nixosModuleTest,
  zramTuningOnlyModuleTest,
  nixosModuleExecuteTest,
  nixosModuleHandoffAutoImportTest,
  nixosModuleBootModeTest,
  nixosModuleInstallModeTest,
  nixosModuleCollisionTest,
  nixosModuleDiskCollisionTest,
  nixosModulePartitionCollisionTest,
  nixosModuleLuksKeyslotCollisionTest,
  nixosModuleLuksTokenCollisionTest,
  nixosModuleBackingFileCollisionTest,
  nixosModuleBtrfsSubvolumeCollisionTest,
  nixosModuleBtrfsQgroupCollisionTest,
  nixosModuleDmMapCollisionTest,
  nixosModuleVdoVolumeCollisionTest,
  nixosModulePhysicalVolumeCollisionTest,
  nixosModuleLoopDeviceCollisionTest,
  nixosModuleMdRaidCollisionTest,
  nixosModuleMultipathMapCollisionTest,
  nixosModuleNvmeNamespaceCollisionTest,
  nixosModuleCacheCollisionTest,
  nixosModulePoolCollisionTest,
  nixosModuleDatasetCollisionTest,
  nixosModuleZvolCollisionTest,
  nixosModuleVolumeGroupCollisionTest,
  nixosModuleVolumeCollisionTest,
  nixosModuleThinPoolCollisionTest,
  nixosModuleLvmCacheCollisionTest,
  nixosModuleSnapshotCollisionTest,
  nixosModuleIscsiSessionCollisionTest,
  nixosModuleLunPathCollisionTest,
}:

{
  inherit diskNix;
  clippy = pkgs.rustPlatform.buildRustPackage {
    pname = "disk-nix-clippy";
    version = "0.1.0";
    src = self;
    cargoLock.lockFile = root + /Cargo.lock;
    nativeBuildInputs = [ pkgs.clippy ];
    buildPhase = ''
      runHook preBuild
      cargo clippy --workspace --all-targets --offline -- -D warnings
      runHook postBuild
    '';
    doCheck = false;
    installPhase = ''
      runHook preInstall
      touch "$out"
      runHook postInstall
    '';
  };
}
// import ./integration-checks.nix args
// import ./documentation-checks.nix args
// import ./nixos-module-checks.nix args
