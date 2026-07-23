{
  pkgs,
  root,
}:

{
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
