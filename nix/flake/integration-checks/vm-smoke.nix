{
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

{
  integrationLayeredVmSmoke = pkgs.runCommand "disk-nix-integration-layered-vm-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-layered-vm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'parted -s "$loopdev" mklabel gpt' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'cryptsetup luksFormat' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'pvcreate --force --yes' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'partitions:layeredPart:grow' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'growpart' ${root + /scripts/integration-layered-vm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'luks.devices:layeredMapper:grow' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'cryptsetup", "resize"' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'volumes:layeredRoot:grow' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvextend", "--resizefs", "--size", "192M"' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'filesystem:layeredRoot:grow' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'resize2fs' ${root + /scripts/integration-layered-vm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'filesystems:layeredRootRemount:remount' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'remount,rw,noatime' ${root + /scripts/integration-layered-vm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'vgchange --activate n' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'luks.devices:layeredMapper:close' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'cryptsetup", "close"' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix layered vm persistence check' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'layeredFailureGrow' ${root + /scripts/integration-layered-vm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'xfs_growfs' ${root + /scripts/integration-layered-vm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'partialExecutionRecovery.completedActionIds' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'partialExecutionRecovery.remainingActionIds' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'rollbackRecipes' ${root + /scripts/integration-layered-vm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'reversibleMutations.commands' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'destructiveMutations.commands' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'requiredTopologyEvidence' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'layeredResumeRemount' ${
      root + /scripts/integration-layered-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'resume-apply.json' ${root + /scripts/integration-layered-vm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'remount,rw,relatime' ${root + /scripts/integration-layered-vm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'fresh topology' ${root + /scripts/integration-layered-vm-smoke.sh}
    touch "$out"
  '';
  integrationDiskoExamples = pkgs.runCommand "disk-nix-integration-disko-examples-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-disko-examples.sh}
    ${pkgs.nodejs}/bin/node --check ${root + /scripts/translate-disko-examples.mjs}
    ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_DISKO_E2E_CONFIRM' ${
      root + /scripts/integration-disko-examples.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_DISKO_E2E_PREFLIGHT' ${
      root + /scripts/integration-disko-examples.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_DISKO_E2E_DEVICES' ${
      root + /scripts/integration-disko-examples.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_DISKO_E2E_REQUIRE_ALL_KERNELS' ${
      root + /scripts/integration-disko-examples.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'wwn-0x5000c500a5a461dc' ${
      root + /scripts/integration-disko-examples.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'remap_devices' ${root + /scripts/integration-disko-examples.sh}
    ${pkgs.gnugrep}/bin/grep -q 'allowed_disk_roots' ${root + /scripts/integration-disko-examples.sh}
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-disko-e2e' ${root + /scripts/integration-disko-examples.sh}
    ${pkgs.gnugrep}/bin/grep -q 'validate_execute_plan_paths' ${
      root + /scripts/integration-disko-examples.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'stand-alone/configuration.nix' ${root + /examples/disko/manifest.json}
    ${pkgs.gnugrep}/bin/grep -q 'zfs-with-vdevs.nix' ${root + /examples/disko/manifest.json}
    DISK_NIX_BIN=${diskNix}/bin/disk-nix \
      DISK_NIX_DISKO_EXAMPLES_DIR=${root + /examples/disko} \
      ${integrationDiskoExamples}/bin/disk-nix-integration-disko-examples
    DISK_NIX_BIN=${diskNix}/bin/disk-nix \
      DISK_NIX_DISKO_EXAMPLES_DIR=${root + /examples/disko} \
      DISK_NIX_DISKO_E2E_PREFLIGHT=1 \
      ${integrationDiskoExamples}/bin/disk-nix-integration-disko-examples
    touch "$out"
  '';
  integrationVmSmoke = pkgs.runCommand "disk-nix-integration-vm-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-vm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_ASSUME_VM ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'systemd-detect-virt --quiet --vm' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'default_harnesses="loop btrfs swap layered-vm failure-recovery"' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-loop-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-swap-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-zram-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-bcache-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-bcachefs-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-mdraid-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-zfs-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-nfs-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-vdo-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-iscsi-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-multipath-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-nvme-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-target-lun-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-failure-recovery-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-integration-layered-vm-smoke' ${
      root + /scripts/integration-vm-smoke.sh
    }
    touch "$out"
  '';
}
