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
  integrationLvmSmoke = pkgs.runCommand "disk-nix-integration-lvm-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-lvm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${root + /scripts/integration-lvm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'pvcreate --force --yes' ${root + /scripts/integration-lvm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'vgcreate' ${root + /scripts/integration-lvm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'lvcreate --yes --type thin-pool' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvcreate --yes --snapshot' ${root + /scripts/integration-lvm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'lvcreate --yes --type cache-pool' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvconvert --yes --type cache --cachepool' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'mkfs.ext4 -F -q "$origin_path"' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix LVM cache sentinel' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'cmp "$sentinel_expected" "$mountpoint/sentinel.txt"' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:" + $origin + ":set-property:lvm.cache-mode' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvchange", "--cachemode", "writethrough"' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:" + $origin + ":remove-device:" + $cachepool' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvconvert", "--uncache", $origin' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:" + $origin + ":add-device:" + $cachepool' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvconvert", "--type", "cache", "--cachepool", $cachepool, $origin' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'cachepool_replacement' ${root + /scripts/integration-lvm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'replaceDevices' ${root + /scripts/integration-lvm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:" + $origin + ":replace-device:" + $old_cachepool' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-lvm-cache-replace' ${root + /scripts/integration-lvm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'vgchange", "--refresh"' ${root + /scripts/integration-lvm-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'thinpools:" + $thinpool + ":rescan' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvmsnapshots:" + $snapshot + ":rescan' ${
      root + /scripts/integration-lvm-smoke.sh
    }
    touch "$out"
  '';
  integrationMdraidSmoke = pkgs.runCommand "disk-nix-integration-mdraid-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-mdraid-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${root + /scripts/integration-mdraid-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'mdadm --create' ${root + /scripts/integration-mdraid-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'replaceDevices' ${root + /scripts/integration-mdraid-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'mdRaids:replacement:replace-device:' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'mdadm", $array, "--replace", $old, "--with", $new' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'mdadm --wait "$array"' ${root + /scripts/integration-mdraid-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'mdadm "$array" --fail "$loop_c"' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'mdadm "$array" --remove "$loop_c"' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'mdadm --examine "$loop_c"' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'stale-member-examine' ${root + /scripts/integration-mdraid-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'failedDetach' ${root + /scripts/integration-mdraid-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'expected failed detach of already-removed MD member' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'mdRaids:failedDetach:remove-device:' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'failedReattach' ${root + /scripts/integration-mdraid-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'expected failed reattach of missing MD member' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'mdRaids:failedReattach:add-device:' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'mdadm", $array, "--add", $missing' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'partialRebuild' ${root + /scripts/integration-mdraid-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'sync_max' ${root + /scripts/integration-mdraid-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'partial-rebuild-sync-completed' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'bounded partial rebuild' ${root + /scripts/integration-mdraid-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'md.degraded-devices' ${root + /scripts/integration-mdraid-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'mdadm", "--detail", "--scan"' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'mdadm", "--examine", "--scan"' ${
      root + /scripts/integration-mdraid-smoke.sh
    }
    touch "$out"
  '';
  integrationZfsSmoke = pkgs.runCommand "disk-nix-integration-zfs-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-zfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-zfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'losetup --find --show' ${root + /scripts/integration-zfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'zpool create' ${root + /scripts/integration-zfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'zpool destroy' ${root + /scripts/integration-zfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'pools:" + $pool + ":set-property:autotrim' ${
      root + /scripts/integration-zfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'zpool", "set", "autotrim=on"' ${
      root + /scripts/integration-zfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'zpool get -H -o value autotrim' ${
      root + /scripts/integration-zfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'zpool", "scrub"' ${root + /scripts/integration-zfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'replaceDevices' ${root + /scripts/integration-zfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'pools:" + $pool + ":replace-device:" + $old' ${
      root + /scripts/integration-zfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'zpool", "replace"' ${root + /scripts/integration-zfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'zpool status -P' ${root + /scripts/integration-zfs-smoke.sh}
    touch "$out"
  '';
  integrationNfsSmoke = pkgs.runCommand "disk-nix-integration-nfs-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-nfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-nfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NFS_SOURCE ${root + /scripts/integration-nfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NFS_EXPORT_PROPERTY ${root + /scripts/integration-nfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NFS_DATA_SURVIVAL ${root + /scripts/integration-nfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'mount -t "$fs_type"' ${root + /scripts/integration-nfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'findmnt", "--json"' ${root + /scripts/integration-nfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'nfsstat", "-m"' ${root + /scripts/integration-nfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'mount", "-o", ("remount,"' ${root + /scripts/integration-nfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix NFS sentinel' ${root + /scripts/integration-nfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'synthetic NFS remount failure for disk-nix data-survival coverage' ${
      root + /scripts/integration-nfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'failed-remount-apply.json' ${root + /scripts/integration-nfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'resumed-remount-apply.json' ${root + /scripts/integration-nfs-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'exports:" + $export_path + ":set-property:options' ${
      root + /scripts/integration-nfs-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'exportfs", "-i", "-o"' ${root + /scripts/integration-nfs-smoke.sh}
    touch "$out"
  '';
  integrationVdoSmoke = pkgs.runCommand "disk-nix-integration-vdo-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-vdo-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-vdo-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_VDO_NAME ${root + /scripts/integration-vdo-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_VDO_WRITE_POLICY ${root + /scripts/integration-vdo-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'vdo status --name' ${root + /scripts/integration-vdo-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'vdostats --human-readable' ${root + /scripts/integration-vdo-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'vdoVolumes:" + $vdo_name + ":set-property:writePolicy' ${
      root + /scripts/integration-vdo-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'vdo", "changeWritePolicy", "--name"' ${
      root + /scripts/integration-vdo-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'vdo", "status", "--name"' ${root + /scripts/integration-vdo-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'vdostats", "--human-readable"' ${
      root + /scripts/integration-vdo-smoke.sh
    }
    touch "$out"
  '';
  integrationIscsiSmoke = pkgs.runCommand "disk-nix-integration-iscsi-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-iscsi-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-iscsi-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_ISCSI_TARGET ${root + /scripts/integration-iscsi-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_LUN_PATH ${root + /scripts/integration-iscsi-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_LUN_DATA_SURVIVAL ${root + /scripts/integration-iscsi-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_LUN_MOUNTPOINT ${root + /scripts/integration-iscsi-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'iscsiadm --mode session' ${root + /scripts/integration-iscsi-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'lsscsi -t -s' ${root + /scripts/integration-iscsi-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'iscsiadm", "--mode", "session", "--rescan"' ${
      root + /scripts/integration-iscsi-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-scsi-rescan' ${root + /scripts/integration-iscsi-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'multipath", "-r"' ${root + /scripts/integration-iscsi-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'lsscsi", "-t", "-s"' ${root + /scripts/integration-iscsi-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix iSCSI LUN sentinel' ${
      root + /scripts/integration-iscsi-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'synthetic iSCSI LUN rescan failure for disk-nix data-survival coverage' ${
      root + /scripts/integration-iscsi-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'failed-lun-apply.json' ${root + /scripts/integration-iscsi-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'resumed-lun-apply.json' ${root + /scripts/integration-iscsi-smoke.sh}
    touch "$out"
  '';
  integrationMultipathSmoke = pkgs.runCommand "disk-nix-integration-multipath-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-multipath-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-multipath-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_MULTIPATH_MAP ${root + /scripts/integration-multipath-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_MULTIPATH_RESIZE ${
      root + /scripts/integration-multipath-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_MULTIPATH_ADD_PATH ${
      root + /scripts/integration-multipath-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_MULTIPATH_REMOVE_PATH ${
      root + /scripts/integration-multipath-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_MULTIPATH_REPLACE_OLD_PATH ${
      root + /scripts/integration-multipath-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_MULTIPATH_REPLACE_NEW_PATH ${
      root + /scripts/integration-multipath-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_MULTIPATH_FLUSH ${
      root + /scripts/integration-multipath-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'multipath -ll' ${root + /scripts/integration-multipath-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'lsscsi -t -s' ${root + /scripts/integration-multipath-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'multipathd", "resize", "map"' ${
      root + /scripts/integration-multipath-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'multipathd", "add", "path"' ${
      root + /scripts/integration-multipath-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'multipathd", "del", "path"' ${
      root + /scripts/integration-multipath-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'replaceDevices' ${root + /scripts/integration-multipath-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'multipathMaps:paths:replace-device:' ${
      root + /scripts/integration-multipath-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'multipath", "-f"' ${root + /scripts/integration-multipath-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'multipath", "-ll"' ${root + /scripts/integration-multipath-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'multipath", "-r"' ${root + /scripts/integration-multipath-smoke.sh}
    touch "$out"
  '';
  integrationNvmeSmoke = pkgs.runCommand "disk-nix-integration-nvme-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-nvme-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NVME_CONTROLLER ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NVME_GROW ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NVME_ATTACH_DETACH ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NVME_CREATE_DELETE ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NVME_RECONNECT ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NVME_RECONNECT_NQN ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NVME_RECONNECT_TRANSPORT ${
      root + /scripts/integration-nvme-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NVME_RECONNECT_TRADDR ${
      root + /scripts/integration-nvme-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NVME_NAMESPACE_ID ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NVME_NAMESPACE_SIZE ${
      root + /scripts/integration-nvme-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_NVME_CONTROLLERS ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'nvme list-ns' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'nvme list-subsys' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'nvme", "list-ns"' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'nvme", "ns-rescan"' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:" + $controller + ":grow' ${
      root + /scripts/integration-nvme-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'list-ns-grown' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:" + $controller + ":create' ${
      root + /scripts/integration-nvme-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:" + $controller + ":destroy' ${
      root + /scripts/integration-nvme-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'nvme", "create-ns"' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'nvme", "delete-ns"' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'list-ns-created' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'list-ns-deleted' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'namespace_present' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'NVMe namespace identity drift' ${
      root + /scripts/integration-nvme-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'nvme disconnect -n "$reconnect_nqn"' ${
      root + /scripts/integration-nvme-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'connect_args=(connect -t "$reconnect_transport"' ${
      root + /scripts/integration-nvme-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'connect_args+=(-s "$reconnect_trsvcid")' ${
      root + /scripts/integration-nvme-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'list-ns-reconnected' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'nvme", "attach-ns"' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'nvme", "detach-ns"' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'list-ns-attached' ${root + /scripts/integration-nvme-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'list-ns-detached' ${root + /scripts/integration-nvme-smoke.sh}
    touch "$out"
  '';
  integrationTargetLunSmoke = pkgs.runCommand "disk-nix-integration-target-lun-smoke-check" { } ''
    ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-target-lun-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'targetcli /backstores/block create' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'targetcli /iscsi create' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'targetLuns' ${root + /scripts/integration-target-lun-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'operation: "attach"' ${root + /scripts/integration-target-lun-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'operation: "detach"' ${root + /scripts/integration-target-lun-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'targetluns:" + $target_iqn + ":attach' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'targetluns:" + $target_iqn + ":detach' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'destroy: true' ${root + /scripts/integration-target-lun-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'targetluns:" + $target_iqn + ":destroy' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'allowDestructive=true' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'lio.writeCache' ${root + /scripts/integration-target-lun-smoke.sh}
    ${pkgs.gnugrep}/bin/grep -q 'emulate_write_cache=0' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix target-side LUN sentinel' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN detach failure for disk-nix data-survival coverage' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'failed-detach-apply.json' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'failed-and-resumed detach data survival' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'target-side LUN integration smoke test' ${
      root + /scripts/integration-target-lun-smoke.sh
    }
    touch "$out"
  '';
  integrationFailureRecoverySmoke =
    pkgs.runCommand "disk-nix-integration-failure-recovery-smoke-check" { }
      ''
        ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-failure-recovery-smoke.sh}
        ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake_tools/lvs' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-grow-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-xfs-grow-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-btrfs-scrub-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-btrfs-rebalance-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-filesystem-trim-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-filesystem-check-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-filesystem-repair-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-swap-label-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-dm-rename-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-zfs-dataset-rename-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-btrfs-snapshot-clone-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-zfs-snapshot-clone-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-vg-rename-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-rollback-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-create-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-grow-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-attach-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-detach-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-attach-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-detach-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-destroy-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-grow-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-property-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-rescan-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-tgt-attach-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-tgt-detach-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-tgt-destroy-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-tgt-grow-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-tgt-property-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-tgt-rescan-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-multipath-replace-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-md-add-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-iscsi-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-iscsi-login-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-format-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-close-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-grow-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-keyslot-add-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-token-import-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-keyslot-remove-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-token-remove-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-multipath-resize-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-attach-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-detach-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-replace-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM cache replacement failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:vg0/root:replace-device:vg0/root-cache' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-rescan-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-nfs-unmount-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-nfs-export-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-nfs-unexport-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-property-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-bcache-replace-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic bcache replacement failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'caches:/dev/bcache0:replace-device:/dev/disk/by-id/old-cache' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-bcache-rescan-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q partialExecutionRecovery ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic resize failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM grow failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-thin-create-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM thin-pool create failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'thinpools:vg0/newpool:create' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-thin-grow-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM thin-pool grow failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'thinpools:vg0/thinpool:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic XFS grow failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic Btrfs scrub failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic Btrfs rebalance failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-btrfs-replace-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic Btrfs device replacement failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-bcachefs-replace-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic bcachefs replacement rereplicate failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:bulk:replace-device:/dev/disk/by-id/old-bcachefs-device' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic filesystem trim failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic filesystem check failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic filesystem repair failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-filesystem-property-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic filesystem property failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:scratch:set-property:label' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic swap label failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-zram-rescan-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic zram rescan failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'zram:rescan' ${root + /scripts/integration-failure-recovery-smoke.sh}
        ${pkgs.gnugrep}/bin/grep -q 'fake-zram-property-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic zram property inventory failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'zram:set-property:algorithm' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-loop-rescan-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic loop rescan failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'loopdevices:/dev/loop7:rescan' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-backing-file-rescan-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic backing-file rescan stat failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'backingfiles:inventory:rescan' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-backing-file-grow-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic backing-file grow truncate failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'backingfiles:root:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-backing-file-create-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic backing-file create truncate failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'backingfiles:new:create' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic device-mapper rename failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic ZFS dataset rename failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic Btrfs snapshot clone failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic ZFS snapshot clone failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM VG rename failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-vg-replace-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM VG replacement pvmove failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-zfs-pool-replace-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic ZFS pool replacement failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'pools:tank:replace-device:/dev/disk/by-id/old-zfs-vdev' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic zfs rollback failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace create failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace grow rescan failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace attach failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace detach failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace delete failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO create failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO attach ACL failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO detach unmap failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO destroy backstore failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'target-side LUN LIO native grow with backing capacity and host verification' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO property failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO rescan inventory failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt create failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt attach bind failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt detach logicalunit failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt destroy target failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'target-side LUN tgt native grow with backing capacity and host verification' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt property failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt rescan inventory failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-scst-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic SCST target-side LUN add_lun failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:scst.root:create' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'run_scst_failure_case' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-scst-$name-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:scst.root:attach' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:scst.root:detach' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:scst.root:destroy' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:scst.root:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetLuns:iqn.2026-06.example:scst.root:set-property:read_only' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:scst.root:rescan' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q '"--mode", "logicalunit", "--op", "update"' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-host-lun-rescan-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic host-side LUN SCSI rescan failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'luns:iqn.2026-06.example:storage/root:0:rescan' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'run_multipath_failure_case' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic multipath add failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'multipathMaps:root-map:add-device:/dev/sdb' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic multipath remove failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'multipathMaps:root-map:remove-device:/dev/sde' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic multipath destroy flush failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'multipathmaps:root-map:destroy' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic multipath resize failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic multipath replace delete failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-md-create-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID create failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'mdraids:newroot:create' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-md-assemble-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID assemble failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'mdraids:existing:assemble' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-md-stop-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID stop failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'mdraids:oldroot:stop' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-md-grow-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID grow failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'mdraids:root:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID add-member failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-md-remove-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID remove-member failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID replace failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS open failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS format failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS close failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS grow failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS keyslot add failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS token import failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS keyslot remove failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS token remove failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-property-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS property failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'luks.devices:cryptroot:set-property:label' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic partition grow failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic NFS remount failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic NFS unmount failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic NFS export failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic NFS unexport failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'exports:share:export' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'exports:oldshare:unexport' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic iscsi logout failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic iscsi login failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-iscsi-rescan-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic iscsi rescan failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'iscsisessions:iqn.2026-06.example:storage.root:rescan' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic lvm cache attach failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic lvm cache detach failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic lvm cache rescan failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-create-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO create failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:new-cache:create' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-rescan-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO rescan stats failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:refresharchive:rescan' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO grow failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-physical-grow-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO physical grow failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:archive-physical:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-start-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO start failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:warmarchive:start' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-stop-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO stop failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:coldarchive:stop' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-remove-tools' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO remove failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:old-cache:destroy' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO property failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic bcache property failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic bcache rescan failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'synthetic lvm cache property failure' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'snapshot:tank/home@before:rollback' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme0:create' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme1:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme2:attach' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme3:detach' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme4:destroy' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:storage.root:create' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:storage.root:attach' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:storage.root:detach' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:storage.root:destroy' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:storage.root:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:tgt.root:create' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:tgt.root:attach' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:tgt.root:detach' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:tgt.root:destroy' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:tgt.root:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:data:replace-device:/dev/disk/by-id/old-btrfs-device' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'multipathmaps:root-map:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'multipathMaps:root-map:replace-device:/dev/sdc' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'mdRaids:root:add-device:/dev/disk/by-id/nvme-spare' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'mdRaids:root:replace-device:/dev/disk/by-id/old-md-member' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'volumeGroups:vg0:replace-device:/dev/disk/by-id/old-pv' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'luks.devices:cryptarchive:open' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'luks.devices:cryptnew:format' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'luks.devices:cryptclosed:close' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'luks.devices:cryptroot:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'lukskeyslots:cryptroot:1:add-key' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'lukstokens:cryptroot:0:import-token' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'lukskeyslots:rootremove:remove-key' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'lukstokens:rootremove:remove-token' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'partitions:root:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'nfs.mounts:/srv/tuned:remount' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'nfs.mounts:/srv/old:unmount' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'iscsisessions:iqn.2026-06.example:storage.old:logout' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'iscsisessions:iqn.2026-06.example:storage.root:login' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:vg0/root:add-device:vg0/root-cache' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:vg0/root:remove-device:vg0/root-cache' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:archive:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'vdoVolumes:archive:set-property:writePolicy' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'caches:writeback-cache:set-property:bcache.cache-mode' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:vg0/root:set-property:lvm.cache-mode' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'completedMutatingCommandCount' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'volumes:root:grow' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'xfs_growfs' ${root + /scripts/integration-failure-recovery-smoke.sh}
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:data:scrub' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:data:rebalance' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:scratch:trim' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:home:check' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:data:repair' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'swaps:primary:set-property:label' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'dmmaps:cryptswap:rename' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'datasets:tank/home:rename' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'snapshot:before-clone:clone:tank/home-review' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'volumegroups:vg-old:rename' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'roll-forward-review' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        ${pkgs.gnugrep}/bin/grep -q 'rollback-review' ${
          root + /scripts/integration-failure-recovery-smoke.sh
        }
        touch "$out"
      '';
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
