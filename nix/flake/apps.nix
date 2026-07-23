{
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
  mkIntegrationApp = package: binary: description: {
    type = "app";
    program = "${package}/bin/${binary}";
    meta = { inherit description; };
  };
in
{
  default = {
    type = "app";
    program = "${diskNix}/bin/disk-nix";
    meta = diskNix.meta;
  };

  integration-loop-smoke =
    mkIntegrationApp integrationLoopSmoke "disk-nix-integration-loop-smoke"
      "Root-only loop-backed disk-nix smoke integration harness";
  integration-btrfs-smoke =
    mkIntegrationApp integrationBtrfsSmoke "disk-nix-integration-btrfs-smoke"
      "Root-only Btrfs loop-backed disk-nix smoke integration harness";
  integration-bcachefs-smoke =
    mkIntegrationApp integrationBcachefsSmoke "disk-nix-integration-bcachefs-smoke"
      "Root-only bcachefs loop-backed disk-nix smoke integration harness";
  integration-bcache-smoke =
    mkIntegrationApp integrationBcacheSmoke "disk-nix-integration-bcache-smoke"
      "Root-only bcache loop-backed disk-nix property mutation harness";
  integration-luks-smoke =
    mkIntegrationApp integrationLuksSmoke "disk-nix-integration-luks-smoke"
      "Root-only LUKS loop-backed disk-nix smoke integration harness";
  integration-swap-smoke =
    mkIntegrationApp integrationSwapSmoke "disk-nix-integration-swap-smoke"
      "Root-only swap loop-backed disk-nix smoke integration harness";
  integration-zram-smoke =
    mkIntegrationApp integrationZramSmoke "disk-nix-integration-zram-smoke"
      "Root-only zram disk-nix property reconciliation harness";
  integration-lvm-smoke =
    mkIntegrationApp integrationLvmSmoke "disk-nix-integration-lvm-smoke"
      "Root-only LVM loop-backed disk-nix smoke integration harness";
  integration-mdraid-smoke =
    mkIntegrationApp integrationMdraidSmoke "disk-nix-integration-mdraid-smoke"
      "Root-only MD RAID loop-backed disk-nix smoke integration harness";
  integration-zfs-smoke =
    mkIntegrationApp integrationZfsSmoke "disk-nix-integration-zfs-smoke"
      "Root-only ZFS loop-backed disk-nix smoke integration harness";
  integration-nfs-smoke =
    mkIntegrationApp integrationNfsSmoke "disk-nix-integration-nfs-smoke"
      "Root-only NFS client disk-nix smoke integration harness";
  integration-vdo-smoke =
    mkIntegrationApp integrationVdoSmoke "disk-nix-integration-vdo-smoke"
      "Root-only VDO disk-nix smoke integration harness";
  integration-iscsi-smoke =
    mkIntegrationApp integrationIscsiSmoke "disk-nix-integration-iscsi-smoke"
      "Root-only iSCSI session disk-nix smoke integration harness";
  integration-multipath-smoke =
    mkIntegrationApp integrationMultipathSmoke "disk-nix-integration-multipath-smoke"
      "Root-only multipath map disk-nix integration harness";
  integration-nvme-smoke =
    mkIntegrationApp integrationNvmeSmoke "disk-nix-integration-nvme-smoke"
      "Root-only NVMe namespace disk-nix smoke integration harness";
  integration-target-lun-smoke =
    mkIntegrationApp integrationTargetLunSmoke "disk-nix-integration-target-lun-smoke"
      "Root-only LIO target-side LUN property integration harness";
  integration-failure-recovery-smoke =
    mkIntegrationApp integrationFailureRecoverySmoke "disk-nix-integration-failure-recovery-smoke"
      "Synthetic failed-apply disk-nix partial recovery smoke integration harness";
  integration-layered-vm-smoke =
    mkIntegrationApp integrationLayeredVmSmoke "disk-nix-integration-layered-vm-smoke"
      "Root-only layered loop/LUKS/LVM/ext4 VM integration harness";
  integration-disko-examples =
    mkIntegrationApp integrationDiskoExamples "disk-nix-integration-disko-examples"
      "Dry-run and guarded destructive disk-nix translations of disko examples";
  integration-vm-smoke =
    mkIntegrationApp integrationVmSmoke "disk-nix-integration-vm-smoke"
      "VM-only destructive disk-nix integration suite orchestrator";
}
