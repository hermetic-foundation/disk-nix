{
  pkgs,
  root,
  diskNix,
}:

rec {
  integrationLoopSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-loop-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.e2fsprogs
      pkgs.jq
      pkgs.util-linux
    ];
    text = builtins.readFile (root + /scripts/integration-loop-smoke.sh);
  };
  integrationBtrfsSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-btrfs-smoke";
    runtimeInputs = [
      diskNix
      pkgs.btrfs-progs
      pkgs.coreutils
      pkgs.gnugrep
      pkgs.jq
      pkgs.util-linux
    ];
    text = builtins.readFile (root + /scripts/integration-btrfs-smoke.sh);
  };
  integrationBcachefsSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-bcachefs-smoke";
    runtimeInputs = [
      diskNix
      pkgs.bcachefs-tools
      pkgs.coreutils
      pkgs.jq
      pkgs.util-linux
    ];
    text = builtins.readFile (root + /scripts/integration-bcachefs-smoke.sh);
  };
  integrationBcacheSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-bcache-smoke";
    runtimeInputs = [
      diskNix
      pkgs.bcache-tools
      pkgs.coreutils
      pkgs.jq
      pkgs.kmod
      pkgs.util-linux
    ];
    text = builtins.readFile (root + /scripts/integration-bcache-smoke.sh);
  };
  integrationLuksSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-luks-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.cryptsetup
      pkgs.gnugrep
      pkgs.jq
      pkgs.util-linux
    ];
    text = builtins.readFile (root + /scripts/integration-luks-smoke.sh);
  };
  integrationSwapSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-swap-smoke";
    runtimeInputs = [
      diskNix
      pkgs.cryptsetup
      pkgs.coreutils
      pkgs.jq
      pkgs.lvm2
      pkgs.mdadm
      pkgs.util-linux
      pkgs.zfs
    ];
    text = builtins.readFile (root + /scripts/integration-swap-smoke.sh);
  };
  integrationZramSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-zram-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.jq
      pkgs.util-linux
    ];
    text = builtins.readFile (root + /scripts/integration-zram-smoke.sh);
  };
  integrationLvmSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-lvm-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.e2fsprogs
      pkgs.jq
      pkgs.lvm2
      pkgs.util-linux
    ];
    text = builtins.readFile (root + /scripts/integration-lvm-smoke.sh);
  };
  integrationMdraidSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-mdraid-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.jq
      pkgs.mdadm
      pkgs.util-linux
    ];
    text = builtins.readFile (root + /scripts/integration-mdraid-smoke.sh);
  };
  integrationZfsSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-zfs-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.jq
      pkgs.util-linux
      pkgs.zfs
    ];
    text = builtins.readFile (root + /scripts/integration-zfs-smoke.sh);
  };
  integrationNfsSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-nfs-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.jq
      pkgs.nfs-utils
      pkgs.util-linux
    ];
    text = builtins.readFile (root + /scripts/integration-nfs-smoke.sh);
  };
  integrationVdoSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-vdo-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.jq
      pkgs.vdo
    ];
    text = builtins.readFile (root + /scripts/integration-vdo-smoke.sh);
  };
  integrationIscsiSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-iscsi-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.gnugrep
      pkgs.jq
      pkgs.lsscsi
      pkgs.multipath-tools
      pkgs.openiscsi
    ];
    text = builtins.readFile (root + /scripts/integration-iscsi-smoke.sh);
  };
  integrationMultipathSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-multipath-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.jq
      pkgs.lsscsi
      pkgs.multipath-tools
    ];
    text = builtins.readFile (root + /scripts/integration-multipath-smoke.sh);
  };
  integrationNvmeSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-nvme-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.jq
      pkgs.nvme-cli
    ];
    text = builtins.readFile (root + /scripts/integration-nvme-smoke.sh);
  };
  integrationTargetLunSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-target-lun-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.e2fsprogs
      pkgs.jq
      pkgs.kmod
      pkgs.targetcli-fb
      pkgs.util-linux
    ];
    text = builtins.readFile (root + /scripts/integration-target-lun-smoke.sh);
  };
  integrationFailureRecoverySmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-failure-recovery-smoke";
    runtimeInputs = [
      diskNix
      pkgs.coreutils
      pkgs.jq
    ];
    text = ''
      export DISK_NIX_FAILURE_RECOVERY_SCENARIO_DIR=${
        root + /scripts/integration-failure-recovery-smoke.d
      }
    ''
    + builtins.readFile (root + /scripts/integration-failure-recovery-smoke.sh);
  };
  integrationLayeredVmSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-layered-vm-smoke";
    runtimeInputs = [
      diskNix
      pkgs.cloud-utils
      pkgs.coreutils
      pkgs.cryptsetup
      pkgs.e2fsprogs
      pkgs.gnugrep
      pkgs.jq
      pkgs.lvm2
      pkgs.parted
      pkgs.util-linux
      pkgs.xfsprogs
    ];
    text = builtins.readFile (root + /scripts/integration-layered-vm-smoke.sh);
  };
  integrationDiskoExamples = pkgs.writeShellApplication {
    name = "disk-nix-integration-disko-examples";
    runtimeInputs = [
      pkgs.bcachefs-tools
      pkgs.btrfs-progs
      pkgs.cryptsetup
      diskNix
      pkgs.coreutils
      pkgs.dosfstools
      pkgs.e2fsprogs
      pkgs.f2fs-tools
      pkgs.gnugrep
      pkgs.jq
      pkgs.kmod
      pkgs.lvm2
      pkgs.mdadm
      pkgs.parted
      pkgs.util-linux
      pkgs.xfsprogs
      pkgs.zfs
    ];
    text = ''
      export DISK_NIX_DISKO_EXAMPLES_DIR="''${DISK_NIX_DISKO_EXAMPLES_DIR:-${root + /examples/disko}}"
    ''
    + builtins.readFile (root + /scripts/integration-disko-examples.sh);
  };
  integrationVmSmoke = pkgs.writeShellApplication {
    name = "disk-nix-integration-vm-smoke";
    runtimeInputs = [
      integrationLoopSmoke
      integrationBtrfsSmoke
      integrationBcacheSmoke
      integrationBcachefsSmoke
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
      pkgs.systemd
    ];
    text = builtins.readFile (root + /scripts/integration-vm-smoke.sh);
  };
}
