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
in
{
  formatter = formatProgram;

  packages = {
    default = diskNix;
    disk-nix = diskNix;
    integration-bcache-smoke = integrationBcacheSmoke;
    integration-bcachefs-smoke = integrationBcachefsSmoke;
    integration-btrfs-smoke = integrationBtrfsSmoke;
    integration-luks-smoke = integrationLuksSmoke;
    integration-swap-smoke = integrationSwapSmoke;
    integration-zram-smoke = integrationZramSmoke;
    integration-lvm-smoke = integrationLvmSmoke;
    integration-mdraid-smoke = integrationMdraidSmoke;
    integration-zfs-smoke = integrationZfsSmoke;
    integration-nfs-smoke = integrationNfsSmoke;
    integration-vdo-smoke = integrationVdoSmoke;
    integration-iscsi-smoke = integrationIscsiSmoke;
    integration-multipath-smoke = integrationMultipathSmoke;
    integration-nvme-smoke = integrationNvmeSmoke;
    integration-target-lun-smoke = integrationTargetLunSmoke;
    integration-failure-recovery-smoke = integrationFailureRecoverySmoke;
    integration-layered-vm-smoke = integrationLayeredVmSmoke;
    integration-disko-examples = integrationDiskoExamples;
    integration-vm-smoke = integrationVmSmoke;
    integration-vm-test = integrationVmTest;
    integration-disko-kernel-examples-vm-test = integrationDiskoKernelExamplesVmTest;
    integration-loop-smoke = integrationLoopSmoke;
  };

  apps = {
    default = {
      type = "app";
      program = "${diskNix}/bin/disk-nix";
      meta = diskNix.meta;
    };
    integration-loop-smoke = {
      type = "app";
      program = "${integrationLoopSmoke}/bin/disk-nix-integration-loop-smoke";
      meta = {
        description = "Root-only loop-backed disk-nix smoke integration harness";
      };
    };
    integration-btrfs-smoke = {
      type = "app";
      program = "${integrationBtrfsSmoke}/bin/disk-nix-integration-btrfs-smoke";
      meta = {
        description = "Root-only Btrfs loop-backed disk-nix smoke integration harness";
      };
    };
    integration-bcachefs-smoke = {
      type = "app";
      program = "${integrationBcachefsSmoke}/bin/disk-nix-integration-bcachefs-smoke";
      meta = {
        description = "Root-only bcachefs loop-backed disk-nix smoke integration harness";
      };
    };
    integration-bcache-smoke = {
      type = "app";
      program = "${integrationBcacheSmoke}/bin/disk-nix-integration-bcache-smoke";
      meta = {
        description = "Root-only bcache loop-backed disk-nix property mutation harness";
      };
    };
    integration-luks-smoke = {
      type = "app";
      program = "${integrationLuksSmoke}/bin/disk-nix-integration-luks-smoke";
      meta = {
        description = "Root-only LUKS loop-backed disk-nix smoke integration harness";
      };
    };
    integration-swap-smoke = {
      type = "app";
      program = "${integrationSwapSmoke}/bin/disk-nix-integration-swap-smoke";
      meta = {
        description = "Root-only swap loop-backed disk-nix smoke integration harness";
      };
    };
    integration-zram-smoke = {
      type = "app";
      program = "${integrationZramSmoke}/bin/disk-nix-integration-zram-smoke";
      meta = {
        description = "Root-only zram disk-nix property reconciliation harness";
      };
    };
    integration-lvm-smoke = {
      type = "app";
      program = "${integrationLvmSmoke}/bin/disk-nix-integration-lvm-smoke";
      meta = {
        description = "Root-only LVM loop-backed disk-nix smoke integration harness";
      };
    };
    integration-mdraid-smoke = {
      type = "app";
      program = "${integrationMdraidSmoke}/bin/disk-nix-integration-mdraid-smoke";
      meta = {
        description = "Root-only MD RAID loop-backed disk-nix smoke integration harness";
      };
    };
    integration-zfs-smoke = {
      type = "app";
      program = "${integrationZfsSmoke}/bin/disk-nix-integration-zfs-smoke";
      meta = {
        description = "Root-only ZFS loop-backed disk-nix smoke integration harness";
      };
    };
    integration-nfs-smoke = {
      type = "app";
      program = "${integrationNfsSmoke}/bin/disk-nix-integration-nfs-smoke";
      meta = {
        description = "Root-only NFS client disk-nix smoke integration harness";
      };
    };
    integration-vdo-smoke = {
      type = "app";
      program = "${integrationVdoSmoke}/bin/disk-nix-integration-vdo-smoke";
      meta = {
        description = "Root-only VDO disk-nix smoke integration harness";
      };
    };
    integration-iscsi-smoke = {
      type = "app";
      program = "${integrationIscsiSmoke}/bin/disk-nix-integration-iscsi-smoke";
      meta = {
        description = "Root-only iSCSI session disk-nix smoke integration harness";
      };
    };
    integration-multipath-smoke = {
      type = "app";
      program = "${integrationMultipathSmoke}/bin/disk-nix-integration-multipath-smoke";
      meta = {
        description = "Root-only multipath map disk-nix smoke integration harness";
      };
    };
    integration-nvme-smoke = {
      type = "app";
      program = "${integrationNvmeSmoke}/bin/disk-nix-integration-nvme-smoke";
      meta = {
        description = "Root-only NVMe namespace disk-nix smoke integration harness";
      };
    };
    integration-target-lun-smoke = {
      type = "app";
      program = "${integrationTargetLunSmoke}/bin/disk-nix-integration-target-lun-smoke";
      meta = {
        description = "Root-only LIO target-side LUN property integration harness";
      };
    };
    integration-failure-recovery-smoke = {
      type = "app";
      program = "${integrationFailureRecoverySmoke}/bin/disk-nix-integration-failure-recovery-smoke";
      meta = {
        description = "Synthetic failed-apply disk-nix partial recovery smoke integration harness";
      };
    };
    integration-layered-vm-smoke = {
      type = "app";
      program = "${integrationLayeredVmSmoke}/bin/disk-nix-integration-layered-vm-smoke";
      meta = {
        description = "Root-only layered loop/LUKS/LVM/ext4 VM integration harness";
      };
    };
    integration-disko-examples = {
      type = "app";
      program = "${integrationDiskoExamples}/bin/disk-nix-integration-disko-examples";
      meta = {
        description = "Dry-run and guarded destructive disk-nix translations of disko examples";
      };
    };
    integration-vm-smoke = {
      type = "app";
      program = "${integrationVmSmoke}/bin/disk-nix-integration-vm-smoke";
      meta = {
        description = "VM-only destructive disk-nix integration suite orchestrator";
      };
    };
  };

  checks = import ./checks.nix {
    inherit
      pkgs
      self
      root
      diskNix
      format
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

  devShells.default = pkgs.mkShell {
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
