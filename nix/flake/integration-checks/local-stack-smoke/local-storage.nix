{
  pkgs,
  root,
}:

{
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
}
