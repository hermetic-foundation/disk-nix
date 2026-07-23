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
  integrationVmTest,
  integrationDiskoKernelExamplesVmTest,
  ...
}:

{
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
}
