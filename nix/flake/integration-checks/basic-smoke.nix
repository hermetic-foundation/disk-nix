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
  integrationLoopSmoke = pkgs.runCommand "disk-nix-integration-loop-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-loop-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-loop-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${root + /scripts/integration-loop-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'losetup --set-capacity' ${root + /scripts/integration-loop-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'backingFiles' ${root + /scripts/integration-loop-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'chmod", "0600"' ${root + /scripts/integration-loop-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'loop.read-only' ${root + /scripts/integration-loop-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'blockdev", "--setro"' ${root + /scripts/integration-loop-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'blockdev", "--setrw"' ${root + /scripts/integration-loop-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'mkfs.ext4' ${root + /scripts/integration-loop-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'resize2fs' ${root + /scripts/integration-loop-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'loopSmokeLabel' ${root + /scripts/integration-loop-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'filesystems:loopSmokeLabel:set-property:label' ${
      root + /scripts/integration-loop-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'e2label' ${root + /scripts/integration-loop-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'disknix-loop' ${root + /scripts/integration-loop-smoke.sh}
    touch "$out"
  '';
  integrationBtrfsSmoke = pkgs.runCommand "disk-nix-integration-btrfs-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-btrfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-btrfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${root + /scripts/integration-btrfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'mkfs.btrfs' ${root + /scripts/integration-btrfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'mount -t btrfs' ${root + /scripts/integration-btrfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'btrfsSmokeLabel' ${root + /scripts/integration-btrfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'filesystems:btrfsSmokeLabel:set-property:label' ${
      root + /scripts/integration-btrfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'btrfs", "filesystem", "label"' ${
      root + /scripts/integration-btrfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disknix-btrfs' ${root + /scripts/integration-btrfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'btrfs", "scrub", "start", "-B"' ${
      root + /scripts/integration-btrfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'replaceDevices' ${root + /scripts/integration-btrfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'filesystems:btrfsReplacement:replace-device:' ${
      root + /scripts/integration-btrfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'btrfs", "replace", "start"' ${
      root + /scripts/integration-btrfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'btrfs filesystem show' ${root + /scripts/integration-btrfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'btrfs replace status' ${root + /scripts/integration-btrfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix Btrfs replacement sentinel' ${
      root + /scripts/integration-btrfs-smoke.sh
    }
    touch "$out"
  '';
  integrationBcachefsSmoke = pkgs.runCommand "disk-nix-integration-bcachefs-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-bcachefs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-bcachefs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${root + /scripts/integration-bcachefs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'bcachefs format' ${root + /scripts/integration-bcachefs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'mount -t bcachefs' ${root + /scripts/integration-bcachefs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'bcachefs", "scrub"' ${root + /scripts/integration-bcachefs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'replaceDevices' ${root + /scripts/integration-bcachefs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'filesystems:bcachefsReplacement:replace-device:' ${
      root + /scripts/integration-bcachefs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'bcachefs", "device", "add"' ${
      root + /scripts/integration-bcachefs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'bcachefs", "data", "rereplicate"' ${
      root + /scripts/integration-bcachefs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'bcachefs", "device", "remove"' ${
      root + /scripts/integration-bcachefs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'bcachefs show-super' ${root + /scripts/integration-bcachefs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix bcachefs replacement sentinel' ${
      root + /scripts/integration-bcachefs-smoke.sh
    }
    touch "$out"
  '';
  integrationBcacheSmoke = pkgs.runCommand "disk-nix-integration-bcache-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-bcache-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'make-bcache -B' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'modprobe bcache' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'caches:bcacheSmoke:set-property:bcache.cache-mode' ${
      root + /scripts/integration-bcache-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'cache_set_uuid=' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'caches:bcacheSmoke:remove-device:' ${
      root + /scripts/integration-bcache-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-bcache-detach' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'caches:bcacheSmoke:add-device:' ${
      root + /scripts/integration-bcache-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-bcache-attach' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'bcacheFailedAttach' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'expected failed bcache cache-set attach' ${
      root + /scripts/integration-bcache-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'caches:bcacheFailedAttach:add-device:' ${
      root + /scripts/integration-bcache-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'replacement_cache_loop=' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'replaceDevices' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'cacheSetUuid' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'caches:bcacheReplacement:replace-device:' ${
      root + /scripts/integration-bcache-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-bcache-replace' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'caches:bcacheSmoke:rescan' ${
      root + /scripts/integration-bcache-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-bcache-property' ${
      root + /scripts/integration-bcache-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-bcache-read' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'dirty_data' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'cache_mode' ${root + /scripts/integration-bcache-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'writethrough' ${root + /scripts/integration-bcache-smoke.sh}
    touch "$out"
  '';
  integrationLuksSmoke = pkgs.runCommand "disk-nix-integration-luks-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-luks-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-luks-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${root + /scripts/integration-luks-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'cryptsetup luksFormat' ${root + /scripts/integration-luks-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'cryptsetup open' ${root + /scripts/integration-luks-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'luksSmokeLabel' ${root + /scripts/integration-luks-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'luks.devices:luksSmokeLabel:set-property:label' ${
      root + /scripts/integration-luks-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'cryptsetup", "config"' ${root + /scripts/integration-luks-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'disknix-luks' ${root + /scripts/integration-luks-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'cryptsetup", "close"' ${root + /scripts/integration-luks-smoke.sh}
    touch "$out"
  '';
  integrationSwapSmoke = pkgs.runCommand "disk-nix-integration-swap-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-swap-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-swap-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${root + /scripts/integration-swap-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'mkswap --label' ${root + /scripts/integration-swap-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'swapSmokeLabel' ${root + /scripts/integration-swap-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'swaps:swapSmokeLabel:set-property:label' ${
      root + /scripts/integration-swap-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'swaplabel", "--label"' ${root + /scripts/integration-swap-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'disknix-swap' ${root + /scripts/integration-swap-smoke.sh}
    touch "$out"
  '';
  integrationZramSmoke = pkgs.runCommand "disk-nix-integration-zram-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-zram-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-zram-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'zram:set-property:algorithm' ${
      root + /scripts/integration-zram-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'zram:set-property:priority' ${
      root + /scripts/integration-zram-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'zramctl", "--bytes", "--raw", "--noheadings", "--output-all"' ${
      root + /scripts/integration-zram-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'swapon", "--show", "--bytes", "--raw"' ${
      root + /scripts/integration-zram-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'services.disk-nix.zram' ${root + /scripts/integration-zram-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'non-mutating property reconciliation' ${
      root + /scripts/integration-zram-smoke.sh
    }
    touch "$out"
  '';
}
