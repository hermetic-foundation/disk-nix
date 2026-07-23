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
}
