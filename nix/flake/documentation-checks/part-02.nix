{
  pkgs,
  self,
  root,
  diskNix,
  integrationDiskoExamples,
  ...
}:

''
  ${pkgs.gnugrep}/bin/grep -q 'disk-nix-bcache-replace' ${root + /docs/developer/integration-tests.md}
  ${pkgs.gnugrep}/bin/grep -q 'caches.bcacheSmoke.removeDevices' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'caches.bcacheFailedAttach.addDevices' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'failed-attach recovery' ${root + /docs/developer/integration-tests.md}
  ${pkgs.gnugrep}/bin/grep -q 'caches.bcacheSmoke.addDevices' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'caches.bcacheSmoke.operation = "rescan"' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'backingFiles.<path>.properties.mode' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'loopDevices.<loop>.properties."loop.read-only"' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'zram.properties.algorithm' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'services.disk-nix.zram' ${root + /docs/developer/integration-tests.md}
  ${pkgs.gnugrep}/bin/grep -q 'targetLuns.<iqn>.properties."lio.writeCache"' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'targetLuns.<iqn>.operation = "attach"' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'targetLuns.<iqn>.operation = "detach"' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'targetLuns.<iqn>.destroy = true' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_LUN_PATH' ${root + /docs/developer/integration-tests.md}
  ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_LUN_DATA_SURVIVAL=1' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'disk-nix-iscsi-lun-sentinel.txt' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'luns.<target>:0.operation = "rescan"' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_MULTIPATH_RESIZE=1' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_MULTIPATH_ADD_PATH' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_MULTIPATH_REMOVE_PATH' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_MULTIPATH_REPLACE_OLD_PATH' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'multipathMaps.paths.replaceDevices' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_MULTIPATH_FLUSH=1' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_NVME_CREATE_DELETE=1' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_NVME_GROW=1' ${root + /docs/developer/integration-tests.md}
  ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_NVME_ATTACH_DETACH=1' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'namespace identity drift' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'nvme create-ns <controller>' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'nvme delete-ns <controller>' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'nvme attach-ns <controller>' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'nvme detach-ns <controller>' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'multipathMaps.resize.operation = "grow"' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'multipathMaps.paths.addDevices' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'multipathMaps.flush.destroy = true' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_VM_HARNESSES=target-lun' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'vdoVolumes.<name>.properties.writePolicy' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'exports.<path>.properties.options' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'DISK_NIX_NFS_DATA_SURVIVAL=1' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'disk-nix-nfs-sentinel.txt' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'mdRaids.<name>.replaceDevices' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'mdadm <array> --replace <old-loop> --with <new-loop>' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'mdadm --examine <removed-loop>' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'mdRaids.<name>.removeDevices' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'failed-detach recovery' ${root + /docs/developer/integration-tests.md}
  ${pkgs.gnugrep}/bin/grep -q 'mdRaids.<name>.addDevices' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'failed-reattach recovery' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'fails and removes one RAID1 member' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'real partial failure' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'rollback review safety' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'failed-and-resumed' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'VM-backed failure-injection apply' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'rollback review stays non-mutating' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'clean follow-up apply' ${root + /docs/developer/integration-tests.md}
  ${pkgs.gnugrep}/bin/grep -q 'partition, LUKS, LVM, filesystem grow, and remount' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'multi-domain apply plan for' ${
    root + /docs/developer/integration-tests.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'reconciliationGroups' "$checklist"
  ${pkgs.gnugrep}/bin/grep -q 'reconciliationGroups' ${root + /docs/developer/planning.md}
  ${pkgs.gnugrep}/bin/grep -q 'partiallySuppressed' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'bracketed IPv6 portals' "$checklist"
  ${pkgs.gnugrep}/bin/grep -q 'CHAP secret redaction' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'iSER/RDMA session transport' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'Real-world iSCSI fixture coverage' "$checklist"
  ${pkgs.gnugrep}/bin/grep -q 'discovery authentication redaction' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'normalizes_multi_portal_discovery_auth_and_lun_churn_fixture' ${
    root + /crates/disk-nix-probe/src/iscsi.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'discovery.sendtargets.auth.authmethod' ${
    root + /crates/disk-nix-probe/src/iscsi.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'iser-rdma0' ${root + /crates/disk-nix-probe/src/iscsi.rs}
  ${pkgs.gnugrep}/bin/grep -q '2001:db8:40::10' ${root + /crates/disk-nix-probe/src/iscsi.rs}
  ${pkgs.gnugrep}/bin/grep -q 'Fibre Channel multipath fixture' "$checklist"
  ${pkgs.gnugrep}/bin/grep -q 'Real-world physical Fibre Channel fixture coverage' "$checklist"
  ${pkgs.gnugrep}/bin/grep -q 'zoning-style fabric/WWPN layouts' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'zoning-style fabric/WWPN layouts' ${root + /docs/user/storage-scope.md}
  ${pkgs.gnugrep}/bin/grep -q 'fibre_channel_zoned_fixture_preserves_adapter_alua_and_failed_paths' ${
    root + /crates/disk-nix-probe/src/lib.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'scsi.fc-target-wwpn' ${root + /crates/disk-nix-probe/src/lsscsi.rs}
  ${pkgs.gnugrep}/bin/grep -q 'NVMe/TCP multipath fixture' "$checklist"
  ${pkgs.gnugrep}/bin/grep -q 'native NVMe namespace paths' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'nvme_tcp_multipath_fixture_preserves_native_path_state' ${
    root + /crates/disk-nix-probe/src/lib.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'uuid.aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee' ${
    root + /crates/disk-nix-probe/src/lib.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'Real-world NVMe-oF fixture coverage' "$checklist"
  ${pkgs.gnugrep}/bin/grep -q 'mixed NVMe-oF fixture' ${root + /docs/user/status.md}
''
