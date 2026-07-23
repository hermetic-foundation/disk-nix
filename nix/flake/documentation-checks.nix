{
  pkgs,
  self,
  root,
  diskNix,
  integrationDiskoExamples,
  ...
}:

{
  documentation = pkgs.runCommand "disk-nix-documentation-check" { } ''
    ${pkgs.nodejs}/bin/node ${root + /scripts/check-docs-legibility.mjs} ${self}
    ${pkgs.nodejs}/bin/node --check ${root + /scripts/render-docs.mjs}
    ${pkgs.nodejs}/bin/node --check ${root + /scripts/check-docs-legibility.mjs}
    checklist=${root + /docs/developer/feature-checklist.md}
    execSources="${root + /crates/disk-nix-exec/src/lib.rs} ${
      root + /crates/disk-nix-exec/src/tests.rs
    }"
    planSources="${root + /crates/disk-nix-plan/src/lib.rs} ${
      root + /crates/disk-nix-plan/src/tests.rs
    }"
    cliSources="${root + /crates/disk-nix-cli/src/main.rs} ${root + /crates/disk-nix-cli/src/tests.rs}"
    ${pkgs.gnugrep}/bin/grep -q 'docs/index.md' ${root + /README.md}
    ${pkgs.gnugrep}/bin/grep -q 'docs/user/user-guide.md' ${root + /README.md}
    ${pkgs.gnugrep}/bin/grep -q 'docs/developer/feature-checklist.md' ${root + /README.md}
    ${pkgs.gnugrep}/bin/grep -q 'docs/user/operator-runbooks.md' ${root + /README.md}
    ${pkgs.gnugrep}/bin/grep -q 'feature-checklist.md' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'operator-runbooks.md' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'node scripts/render-docs.mjs' ${root + /docs/index.md}
    ${pkgs.gnugrep}/bin/grep -q 'Documentation index' ${root + /docs/index.md}
    ${pkgs.gnugrep}/bin/grep -q 'User guide' ${root + /docs/index.md}
    ${pkgs.gnugrep}/bin/grep -q 'Common Workflows' ${root + /docs/user/user-guide.md}
    ${pkgs.gnugrep}/bin/grep -q 'Recover From A Failed Apply' ${root + /docs/user/user-guide.md}
    ${pkgs.gnugrep}/bin/grep -q 'Use The NixOS Module' ${root + /docs/user/user-guide.md}
    ${pkgs.gnugrep}/bin/grep -q 'Hardening beyond the checklist' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'Further integration hardening' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'Status labels:' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'Update rules:' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q '\*\*Finished:\*\*' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q '`Partial`: useful support exists' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q '`Desired`: not implemented yet' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'Operator runbooks for high-risk workflows' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'multi-domain mutation' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'VM-backed failure' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'fresh-topology review' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'rollback-review behavior' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'operator-only guidance instead of automated unsafe rollback' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'MD RAID degraded' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real MD RAID member' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'mdadm <array> --replace <old-loop> --with <new-loop>' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'MD RAID stale-superblock' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'MD RAID failed-detach' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'MD RAID failed-reattach' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'MD RAID partial-rebuild' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'replacement-race coverage' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'missing-member coverage: the loop-backed MD harness' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'layered block/filesystem' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'LVM cache data-survival' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'LVM cache, and multipath-backed stacks' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'replacement data-survival coverage' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'cache-device failure-state coverage' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real bcache read-only' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real bcache cache detach/reattach' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'rescan coverage: the loop-backed bcache harness' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'network-storage scenarios' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'NFS failed-and-resumed remount data-survival' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'iSCSI host-LUN failed-and-resumed rescan data-survival' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'target-side LUN failed-and-resumed' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'lab-backed NVMe namespace create/delete' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'lab-backed NVMe namespace grow' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'lab-backed NVMe namespace attach/detach' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'NVMe namespace identity-drift assertions' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'lab-backed multipath' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'path replacement coverage' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real filesystem' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real LUKS header' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real Btrfs filesystem' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real swap signature' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real ZFS pool' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real ZFS pool device replacement' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real bcachefs member replacement' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real LVM cache' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real bcache property' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real bcache cache replacement' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real loop-device' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real backing-file' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real zram property' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real target-side LUN' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'LIO target-side' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'map/unmap coverage: the loop-backed target LUN harness' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'destroy refusal coverage' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real VDO volume' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'real NFS export' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'e2label' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'cryptsetup config' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'btrfs filesystem label' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'swaplabel' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'zpool set' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'lvchange --cachemode' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-bcache-property' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'blockdev --setro' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'chmod 0600' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'zramctl --bytes --raw --noheadings --output-all' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'emulate_write_cache=0' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'vdo changeWritePolicy' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'exportfs -i' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'ext4 grow plus real' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real LUKS header label mutation' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real Btrfs filesystem label mutation' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real Btrfs filesystem device replacement' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real loop-backed swap label mutation' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real ZFS pool property mutation' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real ZFS pool device replacement' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real LVM cache property mutation' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real LVM cache detach and reattach' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real LVM cache replacement' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'cached-origin ext4 sentinel' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real bcache cache-mode mutation, real bcache cache detach/reattach' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real bcache cache detach/reattach' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real bcache failed-attach recovery' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real bcache cache replacement' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real bcachefs member replacement' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real backing-file mode mutation' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real loop-device read-only mutation' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real zram property reconciliation' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real target-side LUN property mutation' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'target-side LIO map/unmap' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'target-side LUN destroy refusal' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'host-side LUN rescan' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'lab-backed multipath resize' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'lab-backed multipath path add/remove' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'replacement, resize, and flush operations' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'multipath flush with `multipath -f`' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real VDO write-policy mutation' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real NFS export option mutation' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'NFS failed-and-resumed remount data-survival' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'iSCSI host-LUN failed-and-resumed rescan data-survival' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'lab-backed NVMe namespace create/delete' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'lab-backed NVMe namespace grow' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'lab-backed NVMe namespace attach/detach' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'NVMe namespace identity-drift assertions' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'real MD RAID member replacement' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'MD RAID stale-superblock evidence' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'MD RAID failed-detach recovery' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'MD RAID failed-reattach recovery' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'missing-member MD RAID rescan' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'loopSmokeLabel.properties.label' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'luksSmokeLabel.properties.label' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'btrfsSmokeLabel.properties.label' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'filesystems.<name>.replaceDevices' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'bcachefs device add' ${root + /docs/developer/integration-tests.md}
    ${pkgs.gnugrep}/bin/grep -q 'swaps.swapSmokeLabel.properties.label' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'pools.<name>.properties.autotrim' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvmCaches.<vg/lv>.properties.lvm.cache-mode' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvmCaches.<vg/lv>.removeDevices' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvmCaches.<vg/lv>.addDevices' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvmCaches.<vg/lv>.replaceDevices' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'disk-nix-lvm-cache-replace' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'cache sentinel survives' ${root + /docs/developer/integration-tests.md}
    ${pkgs.gnugrep}/bin/grep -q 'caches.bcacheSmoke.properties."bcache.cache-mode"' ${
      root + /docs/developer/integration-tests.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'caches.bcacheReplacement.replaceDevices' ${
      root + /docs/developer/integration-tests.md
    }
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
    ${pkgs.gnugrep}/bin/grep -q 'shared namespace UUID/NGUID identity' ${
      root + /docs/user/storage-scope.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'nvme_of_mixed_fabric_fixture_preserves_sharing_and_path_churn' ${
      root + /crates/disk-nix-probe/src/lib.rs
    }
    ${pkgs.gnugrep}/bin/grep -q 'bbbbbbbb-cccc-dddd-eeee-ffffffffffff' ${
      root + /crates/disk-nix-probe/src/lib.rs
    }
    ${pkgs.gnugrep}/bin/grep -q 'node.identity.uuid' ${root + /crates/disk-nix-probe/src/nvme.rs}
    ${pkgs.gnugrep}/bin/grep -q 'Real-world clustered storage fixture coverage' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'DLM/lvmlockd failure fixture' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'split-brain protection refusal' ${root + /docs/user/storage-scope.md}
    ${pkgs.gnugrep}/bin/grep -q 'clustered_lvm_failure_fixture_preserves_lock_manager_and_split_brain_state' ${
      root + /crates/disk-nix-probe/src/lib.rs
    }
    ${pkgs.gnugrep}/bin/grep -q 'lvm.vg-lock-failure' ${root + /crates/disk-nix-probe/src/lvm.rs}
    ${pkgs.gnugrep}/bin/grep -q 'NFS server/client fixture' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'NFS server/client fixture' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'Real-world server/client NFS fixture coverage' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'client remount drift' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'pNFS layout and' ${root + /docs/user/storage-scope.md}
    ${pkgs.gnugrep}/bin/grep -q 'nfs_server_client_fixture_merges_mount_usage_and_export_policy' ${
      root + /crates/disk-nix-probe/src/lib.rs
    }
    ${pkgs.gnugrep}/bin/grep -q 'nfs.export-option-sec", "krb5p' ${
      root + /crates/disk-nix-probe/src/lib.rs
    }
    ${pkgs.gnugrep}/bin/grep -q 'normalizes_referral_pnfs_remount_and_export_reload_fixture' ${
      root + /crates/disk-nix-probe/src/nfs.rs
    }
    ${pkgs.gnugrep}/bin/grep -q 'nfs.export-option-pnfs' ${root + /crates/disk-nix-probe/src/nfs.rs}
    ${pkgs.gnugrep}/bin/grep -q 'SAS enclosure fixture' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'Real-world hardware enclosure and array fixture coverage' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'vendor LUN metadata' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'SES failure attributes' ${root + /docs/user/storage-scope.md}
    ${pkgs.gnugrep}/bin/grep -q 'hardware_array_fixture_preserves_ses_failures_and_identity_drift' ${
      root + /crates/disk-nix-probe/src/lib.rs
    }
    ${pkgs.gnugrep}/bin/grep -q 'vdisk-prod-77-replaced' ${root + /crates/disk-nix-probe/src/lib.rs}
    ${pkgs.gnugrep}/bin/grep -q 'LVM-backed VDO fixture' "$checklist"
    ${pkgs.gnugrep}/bin/grep -q 'stressed VDO fixture' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'vdo_pressure_fixture_preserves_rebuild_policy_and_failure_state' ${
      root + /crates/disk-nix-probe/src/lib.rs
    }
    ${pkgs.gnugrep}/bin/grep -q 'physical-space pressure' ${root + /docs/user/storage-scope.md}
    ${pkgs.gnugrep}/bin/grep -q 'non-block SES enclosure records' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'LVM-backed VDO fixture' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'active/standby state' ${root + /docs/user/storage-scope.md}
    ${pkgs.gnugrep}/bin/grep -q 'emulate_write_cache' ${root + /docs/developer/planning.md}
    ${pkgs.gnugrep}/bin/grep -q 'emulate_write_cache=0' ${
      root + /scripts/integration-failure-recovery-smoke.sh
    }
    ${pkgs.gnugrep}/bin/grep -q 'tgt property updates render' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'provider = "scst"' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'providerCapabilities' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'provider capability contracts' ${root + /docs/developer/planning.md}
    ${pkgs.gnugrep}/bin/grep -q 'target-lun.capacity.expand' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'target_lun_lio_backing_size_command' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'LIO target-side LUN grow has a native reviewed block' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'target_lun_lio_fileio_grow_forces_backstore_resize_before_refresh' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'backstoreType = "fileio"' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'truncate --size <desiredSize> <source>' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'target_lun_tgt_logical_unit_refresh_command' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'tgt target-side LUN grow has a native reviewed refresh path' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'Generic target LUN verification plans' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'target_lun_generic_host_verification_commands' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'arrayId' ${root + /docs/developer/planning.md}
    ${pkgs.gnugrep}/bin/grep -q 'target-lun.array-id.declared' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_recipes' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'read_only_validation' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'replay_proven_safe_rollback_recipe' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'RollbackExecutionReport' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_refuses_unsafe_sections_and_not_ready_commands' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_refuses_missing_tools_before_running_commands' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_recipe_safety_gates' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'filesystem rollback gates' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'block-stack rollback gates' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'advanced-storage rollback gates' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'network-storage rollback gates' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'required_topology_evidence' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'materialize_rollback_topology_evidence' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'materialize_rollback_topology_payloads' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'replay_proven_safe_rollback_recipe_with_topology_payloads' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'topology_payloads' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_evidence_materializes_from_failed_report_and_fresh_probe' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_binds_full_topology_payloads_to_receipt' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_comparison_refusal_reasons' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_diagnostic_refusal_reasons' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_diagnostic_is_live_use_blocker' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_diagnostic_is_stale_identity_blocker' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_diagnostic_is_idempotency_blocker' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_diagnostic_is_data_loss_risk' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_refuses_divergent_topology_comparison_before_running_commands' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_refuses_risky_topology_diagnostics_before_running_commands' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'topology-already-rolled-back' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_refuses_missing_required_topology_evidence_before_running_commands' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_requires_original_receipt_binding_before_running_commands' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_command_data_loss_risk_reason' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_command_live_use_blocker_reason' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_command_identity_blocker_reason' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollback_command_idempotency_blocker_reason' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'live-use-blocker-metadata' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'ambiguous-stale-identity-metadata' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'idempotency-externally-modified-metadata' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'plausible data-loss command metadata' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback replay refuses missing required tools' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback recipes emit filesystem safety gates' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback recipes emit block-stack safety gates' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback recipes emit advanced-storage safety' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback recipes emit network-storage safety gates' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'metadata advertises already rolled-back' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'idempotency diagnostics for already satisfied' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'detailed post-failure topology diagnostics report divergent' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'ambiguous rollback points and stale identity data' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'behavior for mounted filesystems' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'topology-aware refusal' ${root + /docs/developer/feature-checklist.md}
    ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback recipes declare required topology' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'negative tests proving' ${root + /docs/developer/feature-checklist.md}
    ${pkgs.gnugrep}/bin/grep -q 'not bound to the failed' ${root + /docs/developer/feature-checklist.md}
    ${pkgs.gnugrep}/bin/grep -q 'current topology differs' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'data-loss-prone operations make rollback unsafe' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback replay can materialize deterministic' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'receiptBinding.topologyPayloads' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'crate-level integration' ${root + /docs/developer/feature-checklist.md}
    ${pkgs.gnugrep}/bin/grep -q 'proven_rollback_recipe_replays_and_emits_receipt_binding' ${
      root + /crates/disk-nix-exec/tests/rollback_replay.rs
    }
    ${pkgs.gnugrep}/bin/grep -q 'filesystem_remount_failure_emits_proven_safe_rollback_recipe' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'filesystem_property_failure_emits_proven_safe_rollback_recipe' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'filesystem_check_scrub_and_repair_failures_emit_refused_rollback_recipes' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'block_stack_property_failures_emit_proven_safe_rollback_recipes' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'block_stack_verification_failures_emit_proven_safe_rollback_recipes' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'block_stack_refused_boundaries_emit_operator_only_rollback_recipes' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'block_stack_zram_boundary_emits_refused_rollback_recipe' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'advanced_storage_property_failures_emit_proven_safe_rollback_recipes' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'advanced_storage_refused_boundaries_emit_operator_only_rollback_recipes' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'network_storage_failures_emit_proven_safe_rollback_recipes' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'network_storage_refused_boundaries_emit_operator_only_rollback_recipes' $execSources
    ${pkgs.gnugrep}/bin/grep -q 'rollbackOptions' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'rollbackValue' ${root + /docs/developer/planning.md}
    ${pkgs.gnugrep}/bin/grep -q 'device-mapper rename verification failures' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'Block-stack property declarations use the same' ${
      root + /docs/developer/planning.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'Advanced-storage declarations also use' ${
      root + /docs/developer/planning.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'ZFS snapshot rollback/clone' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'Network-storage declarations also use' ${
      root + /docs/developer/planning.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'Network-storage failures can also produce proven-safe recipes' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback replay refuses proven-safe recipes when' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'commands whose metadata advertises ambiguous rollback points' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'commands whose metadata advertises active consumers' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback replay refuses reversible mutation' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'rollbackRecipes' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'requiredTopologyEvidence' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'replay_proven_safe_rollback_recipe_with_topology_evidence' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'topology comparison summary already has missing targets' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'open encrypted mappings, active' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'ambiguous rollback points, ambiguous rollback targets' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'Idempotency' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'operatorOnlyHandoff' ${root + /docs/user/cli.md}
    ${pkgs.gnugrep}/bin/grep -q 'proven-safe reversible rollback' ${root + /docs/user/status.md}
    ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback has an execution engine' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback replay refuses' ${
      root + /docs/developer/feature-checklist.md
    }
    ${pkgs.gnugrep}/bin/grep -q 'scstadmin' ${root + /docs/developer/planning.md}
    ${pkgs.gnugrep}/bin/grep -q 'initiatorGroup' ${root + /docs/developer/planning.md}
    runbooks=${root + /docs/user/operator-runbooks.md}
    for runbook in \
      "Device replacement" \
      Rollback \
      "Failed apply recovery" \
      "Degraded arrays and pools" \
      "Shared storage and network storage" \
      "Change record"
    do
      ${pkgs.gnugrep}/bin/grep -q "^## $runbook$" "$runbooks"
    done
    for section in \
      Foundation \
      "Read-only storage awareness" \
      "Planning and apply safety" \
      "Lifecycle operations" \
      "Current-topology reconciliation" \
      "Recovery guidance" \
      "NixOS integration" \
      "Testing and proof" \
      Documentation
    do
      ${pkgs.gnugrep}/bin/grep -q "^## $section$" "$checklist"
    done
    touch "$out"
  '';
  examples = pkgs.runCommand "disk-nix-examples-check" { nativeBuildInputs = [ pkgs.jq ]; } ''
    simplePlan=$(mktemp)
    lifecyclePlan=$(mktemp)
    simpleApply=$(mktemp)
    lifecycleApply=$(mktemp)
    lifecycleValidate=$(mktemp)
    lifecycleApplyReport=$(mktemp)
    lifecycleValidateReport=$(mktemp)
    emptySpec=$(mktemp)
    emptyExecute=$(mktemp)
    legacySpec=$(mktemp)
    legacyMigration=$(mktemp)
    preflightStatus=$(mktemp)
    schema=$(mktemp)
    scriptOut=$(mktemp)

    ${diskNix}/bin/disk-nix --help | grep -- 'usage'
    ${diskNix}/bin/disk-nix --help | grep -- 'encryption'
    ${diskNix}/bin/disk-nix --help | grep -- 'complex-filesystems'
    ${diskNix}/bin/disk-nix --help | grep -- 'zfs'
    ${diskNix}/bin/disk-nix --help | grep -- 'cache'
    ${diskNix}/bin/disk-nix --help | grep -- 'lvm'
    ${diskNix}/bin/disk-nix --help | grep -- 'vdo'
    ${diskNix}/bin/disk-nix --help | grep -- 'multipath'
    ${diskNix}/bin/disk-nix --help | grep -- 'nvme'
    ${diskNix}/bin/disk-nix --help | grep -- 'raid'
    ${diskNix}/bin/disk-nix --help | grep -- 'loop'
    ${diskNix}/bin/disk-nix --help | grep -- 'swap'
    ${diskNix}/bin/disk-nix --help | grep -- 'iscsi'
    ${diskNix}/bin/disk-nix --help | grep -- 'nfs'
    ${diskNix}/bin/disk-nix probe-status --help | grep -- '--preflight'
    ${diskNix}/bin/disk-nix probe-status --preflight --json > "$preflightStatus"
    jq -e '
      (.environment | has("toolVersions"))
      and (.preflightChecks | has("status"))
      and (.preflightChecks | has("root"))
      and (.preflightChecks | has("unavailableToolCount"))
      and (.preflightChecks | has("failedToolCount"))
      and (.preflightChecks.missingTools | type == "array")
      and (.preflightChecks.failedTools | type == "array")
      and (.preflightChecks.remediation | type == "array")
      and (.preflightChecks.adapterRemediation | type == "array")
      and (.preflightChecks.adapterRemediation | any(.adapter == "nvme-id-ns" and .canonicalAdapter == "nvme" and (.nixPackages | index("pkgs.nvme-cli") != null)))
      and (.preflightChecks.adapterRemediation | any(.adapter == "mdadm-scan" and .canonicalAdapter == "mdraid" and (.nixPackages | index("pkgs.mdadm") != null)))
      and (.preflightChecks.adapterRemediation | any(.adapter == "zramctl" and .canonicalAdapter == "zram" and (.tools | index("zramctl") != null)))
      and (.reports | type == "array")
    ' "$preflightStatus"
    if grep -R -E 'executor-unavailable|does not mutate storage yet|future mutating executor|future `btrfs device remove`|does not run mutating storage commands directly|non-executed command' ${root + /README.md} ${root + /docs}; then
      echo "stale executor documentation found" >&2
      exit 1
    fi
    ${diskNix}/bin/disk-nix schema > "$schema"
    cmp "$schema" ${diskNix}/share/disk-nix/schema/disk-nix-spec.schema.json
    cat > "$legacySpec" <<'EOF'
    {
      "fileSystems": {
        "root": {
          "mountpoint": "/",
          "fsType": "ext4"
        }
      },
      "swapDevices": {
        "swap": {
          "device": "/dev/disk/by-label/swap",
          "operation": "rescan"
        }
      },
      "luksDevices": {
        "cryptroot": {
          "device": "/dev/disk/by-id/luks-root",
          "operation": "open"
        }
      },
      "nfsMounts": {
        "/srv/shared": {
          "source": "nas.example.com:/srv/shared",
          "operation": "mount"
        }
      },
      "iscsiSessions": {
        "iqn.2026-06.example:storage.root": {
          "portal": "192.0.2.10:3260",
          "operation": "login"
        }
      }
    }
    EOF
    ${diskNix}/bin/disk-nix migrate --spec "$legacySpec" --json > "$legacyMigration"
    jq -e '
      .targetVersion == 1
      and .migrated == true
      and .spec.version == 1
      and (.spec | has("fileSystems") | not)
      and (.spec | has("swapDevices") | not)
      and (.spec | has("luksDevices") | not)
      and (.spec | has("nfsMounts") | not)
      and (.spec | has("iscsiSessions") | not)
      and .spec.filesystems.root.mountpoint == "/"
      and .spec.swaps.swap.operation == "rescan"
      and .spec.luks.devices.cryptroot.operation == "open"
      and .spec.nfs.mounts."/srv/shared".source == "nas.example.com:/srv/shared"
      and .spec.iscsi.sessions."iqn.2026-06.example:storage.root".operation == "login"
      and (.changes | any(. == "mapped legacy field fileSystems to filesystems"))
      and (.changes | any(. == "mapped legacy field luksDevices to luks.devices"))
      and (.legacyMappings | any(.source == "fileSystems" and .target == "filesystems" and .scope == "top-level"))
      and (.legacyMappings | any(.source == "spec.fileSystems" and .target == "spec.filesystems" and .scope == "spec"))
      and (.legacyMappings | any(.source == "iscsiSessions" and .target == "iscsi.sessions" and .scope == "top-level"))
      and (.appliedMappings | length == 5)
      and (.appliedMappings | any(.source == "fileSystems" and .target == "filesystems" and .scope == "top-level"))
      and (.appliedMappings | any(.source == "luksDevices" and .target == "luks.devices" and .scope == "top-level"))
      and (.appliedMappings | any(.source == "iscsiSessions" and .target == "iscsi.sessions" and .scope == "top-level"))
    ' "$legacyMigration"
    jq -e '
      ."$schema" == "https://json-schema.org/draft/2020-12/schema"
      and .properties.version.const == 1
      and .properties.spec["$ref"] == "#/$defs/specBody"
      and .properties.apply["$ref"] == "#/$defs/applyPolicy"
      and .properties.swaps["$ref"] == "#/$defs/lifecycleMap"
      and .properties.zram["$ref"] == "#/$defs/zramSpec"
      and ."$defs".specBody.properties.version.const == 1
      and ."$defs".specBody.properties.zram["$ref"] == "#/$defs/zramSpec"
      and ."$defs".zramSpec.properties.operation["$ref"] == "#/$defs/operation"
      and ."$defs".zramSpec.properties.swapDevices.minimum == 1
      and .properties.luks["$ref"] == "#/$defs/luksSpec"
      and .properties.nfs["$ref"] == "#/$defs/nfsSpec"
      and .properties.iscsi["$ref"] == "#/$defs/iscsiSpec"
      and .properties.disks["$ref"] == "#/$defs/lifecycleMap"
      and .properties.partitions["$ref"] == "#/$defs/lifecycleMap"
      and .properties.btrfsSubvolumes["$ref"] == "#/$defs/lifecycleMap"
      and .properties.btrfsQgroups["$ref"] == "#/$defs/lifecycleMap"
      and .properties.vdoVolumes["$ref"] == "#/$defs/lifecycleMap"
      and .properties.physicalVolumes["$ref"] == "#/$defs/lifecycleMap"
      and .properties.luksKeyslots["$ref"] == "#/$defs/lifecycleMap"
      and .properties.luksTokens["$ref"] == "#/$defs/lifecycleMap"
      and .properties.volumes["$ref"] == "#/$defs/lifecycleMap"
      and .properties.volumeGroups["$ref"] == "#/$defs/lifecycleMap"
      and .properties.zvols["$ref"] == "#/$defs/lifecycleMap"
      and .properties.thinPools["$ref"] == "#/$defs/lifecycleMap"
      and .properties.lvmSnapshots["$ref"] == "#/$defs/lifecycleMap"
      and .properties.lvmCaches["$ref"] == "#/$defs/lifecycleMap"
      and .properties.loopDevices["$ref"] == "#/$defs/lifecycleMap"
      and .properties.backingFiles["$ref"] == "#/$defs/lifecycleMap"
      and .properties.dmMaps["$ref"] == "#/$defs/lifecycleMap"
      and .properties.mdRaids["$ref"] == "#/$defs/lifecycleMap"
      and .properties.multipathMaps["$ref"] == "#/$defs/lifecycleMap"
      and .properties.pools["$ref"] == "#/$defs/lifecycleMap"
      and .properties.datasets["$ref"] == "#/$defs/lifecycleMap"
      and .properties.luns["$ref"] == "#/$defs/lifecycleMap"
      and .properties.nvmeNamespaces["$ref"] == "#/$defs/lifecycleMap"
      and .properties.iscsiSessions["$ref"] == "#/$defs/lifecycleMap"
      and .properties.exports["$ref"] == "#/$defs/lifecycleMap"
      and .properties.caches["$ref"] == "#/$defs/lifecycleMap"
      and .properties.snapshots["$ref"] == "#/$defs/snapshotMap"
      and ."$defs".lifecycleObject.properties.physicalSize.type == ["string", "number"]
      and ."$defs".lifecycleObject.properties.vdoPhysicalSize.type == ["string", "number"]
      and ."$defs".lifecycleObject.properties.provider.type == "string"
      and ."$defs".lifecycleObject.properties.storageProvider.type == "string"
      and ."$defs".lifecycleObject.properties.arrayProvider.type == "string"
      and ."$defs".lifecycleObject.properties.arrayId.type == "string"
      and ."$defs".lifecycleObject.properties.storagePool.type == "string"
      and ."$defs".lifecycleObject.properties.volumeId.type == "string"
      and ."$defs".lifecycleObject.properties.snapshotId.type == "string"
      and ."$defs".lifecycleObject.properties.cloneSource.type == "string"
      and ."$defs".lifecycleObject.properties.maskingGroup.type == "string"
      and ."$defs".lifecycleObject.properties.lun.type == ["string", "number"]
      and ."$defs".snapshot.properties.operation["$ref"] == "#/$defs/operation"
      and ."$defs".snapshot.properties.action["$ref"] == "#/$defs/operation"
      and (."$defs".operation.enum | index("grow") != null)
      and (."$defs".operation.enum | index("check") != null)
      and (."$defs".operation.enum | index("repair") != null)
      and (."$defs".operation.enum | index("scrub") != null)
      and (."$defs".operation.enum | index("trim") != null)
      and (."$defs".operation.enum | index("rescan") != null)
      and (."$defs".operation.enum | index("replace-device") != null)
      and (."$defs".operation.enum | index("add-key") != null)
      and (."$defs".operation.enum | index("remove-key") != null)
      and (."$defs".operation.enum | index("import-token") != null)
            and (."$defs".operation.enum | index("remove-token") != null)
            and (."$defs".operation.enum | index("clone") != null)
            and (."$defs".specBody.properties.luks["$ref"] == "#/$defs/luksSpec")
      and (."$defs".specBody.properties.nfs["$ref"] == "#/$defs/nfsSpec")
      and (."$defs".specBody.properties.iscsi["$ref"] == "#/$defs/iscsiSpec")
      and (."$defs".specBody.properties.disks["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.btrfsSubvolumes["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.btrfsQgroups["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.vdoVolumes["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.physicalVolumes["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.luksKeyslots["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.luksTokens["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.volumes["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.volumeGroups["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.zvols["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.thinPools["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.lvmSnapshots["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.lvmCaches["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.loopDevices["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.backingFiles["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.dmMaps["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.mdRaids["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.multipathMaps["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.pools["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.datasets["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.luns["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.targetLuns["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.nvmeNamespaces["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.iscsiSessions["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.exports["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.caches["$ref"] == "#/$defs/lifecycleMap")
      and (."$defs".specBody.properties.snapshots["$ref"] == "#/$defs/snapshotMap")
      and ."$defs".snapshot.properties.operation["$ref"] == "#/$defs/operation"
      and ."$defs".snapshot.properties.action["$ref"] == "#/$defs/operation"
      and ."$defs".snapshot.properties.path.type == "string"
      and ."$defs".snapshot.properties.snapshotPath.type == "string"
      and ."$defs".snapshot.properties.readOnly.type == "boolean"
      and ."$defs".snapshot.properties.readonly.type == "boolean"
      and ."$defs".snapshot.properties.cloneTo.type == "string"
      and ."$defs".snapshot.properties.recursiveRollback.type == "boolean"
      and ."$defs".snapshot.properties."zfs.rollbackRecursive".type == "boolean"
      and (."$defs".operation.enum | index("promote") != null)
      and (."$defs".operation.enum | index("import") != null)
      and (."$defs".operation.enum | index("export") != null)
      and (."$defs".operation.enum | index("unexport") != null)
      and (."$defs".operation.enum | index("attach") != null)
      and (."$defs".operation.enum | index("detach") != null)
      and (."$defs".operation.enum | index("activate") != null)
      and (."$defs".operation.enum | index("deactivate") != null)
      and (."$defs".operation.enum | index("assemble") != null)
      and (."$defs".operation.enum | index("start") != null)
      and (."$defs".operation.enum | index("stop") != null)
      and (."$defs".operation.enum | index("login") != null)
      and (."$defs".operation.enum | index("logout") != null)
      and (."$defs".operation.enum | index("open") != null)
      and (."$defs".operation.enum | index("close") != null)
      and (."$defs".operation.enum | index("mount") != null)
      and (."$defs".operation.enum | index("unmount") != null)
      and (."$defs".operation.enum | index("remount") != null)
      and ."$defs".filesystem.properties.device.type == "string"
      and ."$defs".filesystem.properties.operation["$ref"] == "#/$defs/operation"
      and ."$defs".filesystem.properties.action["$ref"] == "#/$defs/operation"
      and ."$defs".filesystem.properties.neededForBoot.type == "boolean"
      and ."$defs".filesystem.properties.destroy.type == "boolean"
      and ."$defs".filesystem.properties.properties.type == "object"
      and ."$defs".filesystem.properties.metadata.type == "object"
      and ."$defs".filesystem.properties.addDevices.type == "array"
      and ."$defs".filesystem.properties.removeDevices.type == "array"
      and ."$defs".filesystem.properties.replaceDevices.type == "object"
      and ."$defs".lifecycleObject.properties.cacheSetUuid.type == "string"
      and ."$defs".lifecycleObject.properties.cacheSetUUID.type == "string"
      and ."$defs".lifecycleObject.properties."cache-set-uuid".type == "string"
      and ."$defs".lifecycleObject.properties.cache_set_uuid.type == "string"
      and ."$defs".luksSpec.properties.devices["$ref"] == "#/$defs/lifecycleMap"
      and ."$defs".nfsSpec.properties.mounts["$ref"] == "#/$defs/nfsMountMap"
      and ."$defs".nfsMount.properties.source.type == "string"
      and ."$defs".nfsMount.properties.operation["$ref"] == "#/$defs/operation"
      and ."$defs".nfsMount.properties.action["$ref"] == "#/$defs/operation"
      and ."$defs".nfsMount.properties.destroy.type == "boolean"
      and ."$defs".nfsMount.properties.options.type == "array"
      and ."$defs".nfsMount.properties.metadata.type == "object"
      and ."$defs".iscsiSpec.properties.sessions["$ref"] == "#/$defs/lifecycleMap"
      and ."$defs".iscsiSpec.properties.boot["$ref"] == "#/$defs/iscsiBoot"
      and ."$defs".iscsiBoot.properties.loginAll.type == "boolean"
      and (."$defs".iscsiBoot.properties.extraConfig.type | index("null") != null)
      and ."$defs".lifecycleObject.properties.action["$ref"] == "#/$defs/operation"
      and ."$defs".lifecycleObject.properties.renameTo.type == "string"
      and ."$defs".lifecycleObject.properties.renameTarget.type == "string"
      and ."$defs".lifecycleObject.properties.newName.type == "string"
      and ."$defs".lifecycleObject.properties.readOnly.type == "boolean"
      and ."$defs".lifecycleObject.properties.readonly.type == "boolean"
      and ."$defs".lifecycleObject.properties.partitionType.type == "string"
      and (."$defs".lifecycleObject.properties.partitionNumber.type | index("string") != null)
      and (."$defs".lifecycleObject.properties.partitionNumber.type | index("number") != null)
      and (."$defs".lifecycleObject.properties.number.type | index("string") != null)
      and (."$defs".lifecycleObject.properties.startOffset.type | index("number") != null)
      and (."$defs".lifecycleObject.properties.endOffset.type | index("string") != null)
      and ."$defs".lifecycleObject.properties.level.type == "string"
      and ."$defs".lifecycleObject.properties.raidLevel.type == "string"
      and ."$defs".lifecycleObject.properties.devices.type == "array"
      and ."$defs".lifecycleObject.properties.paths.type == "array"
      and ."$defs".lifecycleObject.properties.devicePaths.type == "array"
      and ."$defs".lifecycleObject.properties.path.type == "string"
      and ."$defs".lifecycleObject.properties.client.type == "string"
      and ."$defs".lifecycleObject.properties.portal.type == "string"
      and (."$defs".lifecycleObject.properties.namespaceId.type | index("string") != null)
      and (."$defs".lifecycleObject.properties.nsid.type | index("string") != null)
      and ."$defs".lifecycleObject.properties.controllers.type == "string"
      and (."$defs".lifecycleObject.properties.controllerId.type | index("string") != null)
      and (."$defs".lifecycleObject.properties.controller.type | index("string") != null)
      and (."$defs".lifecycleObject.properties.keySlot.type | index("string") != null)
      and (."$defs".lifecycleObject.properties."key-slot".type | index("string") != null)
      and (."$defs".lifecycleObject.properties.slot.type | index("string") != null)
      and ."$defs".lifecycleObject.properties.keyFile.type == "string"
      and ."$defs".lifecycleObject.properties."key-file".type == "string"
      and ."$defs".lifecycleObject.properties.currentKeyFile.type == "string"
      and ."$defs".lifecycleObject.properties.newKeyFile.type == "string"
      and ."$defs".lifecycleObject.properties."new-key-file".type == "string"
      and (."$defs".lifecycleObject.properties.tokenId.type | index("string") != null)
      and (."$defs".lifecycleObject.properties."token-id".type | index("string") != null)
      and (."$defs".lifecycleObject.properties.token.type | index("string") != null)
      and ."$defs".lifecycleObject.properties.tokenFile.type == "string"
      and ."$defs".lifecycleObject.properties."token-file".type == "string"
      and ."$defs".lifecycleObject.properties.jsonFile.type == "string"
      and ."$defs".lifecycleObject.properties.options.type == "string"
      and ."$defs".applyPolicy.properties.failOnBlocked.default == true
      and ."$defs".applyPolicy.properties.allowPotentialDataLoss.default == false
      and (."$defs".applyPolicy.properties.reportOut.type | index("string") != null)
      and (."$defs".applyPolicy.properties.receiptOut.type | index("string") != null)
    ' "$schema"

    ${diskNix}/bin/disk-nix plan --spec ${root + /examples/simple-root.json} --json > "$simplePlan"
    jq -e '
      .summary.actionCount == 1
      and .summary.offlineRequiredCount == 0
      and .summary.destructiveCount == 0
      and .summary.potentialDataLossCount == 0
      and .summary.unsupportedCount == 0
      and .actions[0].id == "filesystem:root:grow"
      and .dependencyOrder[0].actionId == "filesystem:root:grow"
      and .dependencyOrder[0].phase == "mutate-in-place"
      and .dependencyOrder[0].direction == "lower-layers-first"
      and .dependencyOrder[0].layerRank == 90
      and .actions[0].operation == "grow"
      and .actions[0].risk == "online"
      and .actions[0].context.desiredSize == "100%"
    ' "$simplePlan"

    ${diskNix}/bin/disk-nix plan --spec ${
      root + /examples/lifecycle-update.json
    } --json > "$lifecyclePlan"
    jq -e '
      .summary.actionCount == 105
      and (.dependencyOrder | length) == .summary.actionCount
      and (.dependencyOrder | any(.actionId == "datasets:tank/home:create" and (.unblocks | index("snapshot:tank/home@before-upgrade:create") != null)))
      and (.dependencyOrder | any(.actionId == "snapshot:tank/home@before-upgrade:create" and (.dependsOn | index("datasets:tank/home:create") != null)))
      and (.dependencyOrder | any(.actionId == "btrfssubvolumes:/mnt/persist/@home:create" and (.unblocks | index("snapshot:/mnt/persist/@home-inventory:rescan") != null)))
      and (.dependencyOrder | any(.actionId == "snapshot:/mnt/persist/@home-inventory:rescan" and (.dependsOn | index("btrfssubvolumes:/mnt/persist/@home:create") != null)))
      and .summary.offlineRequiredCount == 31
      and .summary.destructiveCount == 4
      and .summary.potentialDataLossCount == 4
      and .summary.unsupportedCount == 0
      and (.actions | any(.id == "filesystems:home-check:check" and .risk == "offline-required"))
      and (.actions | any(.id == "filesystems:data-scrub:scrub" and .risk == "online"))
      and (.actions | any(.id == "filesystems:scratch-trim:trim" and .risk == "online"))
      and (.actions | any(.id == "filesystems:scratch-remount:remount" and .risk == "online"))
      and (.actions | any(.id == "btrfssubvolumes:/mnt/persist/@home:create" and .risk == "online"))
      and (.actions | any(.id == "btrfssubvolumes:/mnt/persist/@inventory:rescan" and .risk == "online"))
      and (.actions | any(.id == "btrfssubvolumes:/mnt/persist/@old-name:rename" and .risk == "offline-required"))
      and (.actions | any(.id == "btrfsQgroups:0/257:set-property:limit" and .risk == "safe"))
      and (.actions | any(.id == "btrfsQgroups:0/257:set-property:maxExclusive" and .risk == "safe"))
      and (.actions | any(.id == "btrfsqgroups:0/258:rescan" and .risk == "online"))
      and (.actions | any(.id == "volumes:vg0/scratch:create" and .risk == "online"))
      and (.actions | any(.id == "volumes:vg0/archive:deactivate" and .risk == "offline-required"))
      and (.actions | any(.id == "volumes:vg0/reporting:rescan" and .risk == "online"))
      and (.actions | any(.id == "vdovolumes:archive:grow" and .risk == "online"))
      and (.actions | any(.id == "vdovolumes:warmarchive:start" and .risk == "offline-required"))
      and (.actions | any(.id == "vdovolumes:coldarchive:stop" and .risk == "offline-required"))
      and (.actions | any(.id == "vdoVolumes:archive:set-property:writePolicy" and .risk == "safe"))
      and (.actions | any(.id == "vdoVolumes:archive:set-property:compression" and .risk == "safe"))
      and (.actions | any(.id == "vdoVolumes:archive:set-property:deduplication" and .risk == "safe"))
      and (.actions | any(.id == "physicalvolumes:/dev/disk/by-id/nvme-pv-grow:grow" and .risk == "online"))
      and (.actions | any(.id == "lukskeyslots:cryptroot:1:add-key" and .risk == "offline-required"))
      and (.actions | any(.id == "lukskeyslots:cryptroot:2:remove-key" and .risk == "potential-data-loss"))
      and (.actions | any(.id == "lukstokens:cryptroot:0:import-token" and .risk == "offline-required"))
      and (.actions | any(.id == "lukstokens:cryptroot:1:remove-token" and .risk == "potential-data-loss"))
      and (.actions | any(.id == "iscsisessions:iqn.2026-06.example:storage.login:login" and .risk == "online"))
      and (.actions | any(.id == "iscsisessions:iqn.2026-06.example:storage.logout:logout" and .risk == "offline-required"))
      and (.actions | any(.id == "iscsisessions:iqn.2026-06.example:storage.rescan:rescan" and .risk == "online"))
      and (.actions | any(.id == "luns:iqn.2026-06.example:storage/new:2:attach" and .risk == "online"))
      and (.actions | any(.id == "luns:iqn.2026-06.example:storage/old:3:detach" and .risk == "offline-required"))
      and (.actions | any(.id == "luns:iqn.2026-06.example:storage/rescan:4:rescan" and .risk == "online"))
      and (.actions | any(.id == "zvols:tank/vm/root:grow" and .risk == "online"))
      and (.actions | any(.id == "zvols:tank/vm/inventory:rescan" and .risk == "online"))
      and (.actions | any(.id == "thinpools:vg0/thinpool:grow" and .risk == "online"))
      and (.actions | any(.id == "thinpools:vg0/newthin:create" and .risk == "online"))
      and (.actions | any(.id == "thinpools:vg0/reporting:rescan" and .risk == "online"))
      and (.actions | any(.id == "lvmsnapshots:vg0/root-snap:snapshot" and .risk == "reversible"))
      and (.actions | any(.id == "lvmsnapshots:vg0/root-inspect:rescan" and .risk == "online"))
      and (.actions | any(.id == "lvmcaches:vg0/root:create" and .risk == "offline-required"))
      and (.actions | any(.id == "lvmCaches:vg0/root:set-property:lvm.cache-mode" and .risk == "safe"))
      and (.actions | any(.id == "lvmcaches:vg0/archive:rescan" and .risk == "online"))
      and (.actions | any(.id == "loopdevices:/dev/loop7:create" and .risk == "online"))
      and (.actions | any(.id == "loopdevices:/dev/loop10:rescan" and .risk == "online"))
      and (.actions | any(.id == "backingfiles:/var/lib/images/new.img:create" and .risk == "online"))
      and (.actions | any(.id == "backingfiles:/var/lib/images/root.img:grow" and .risk == "online"))
      and (.actions | any(.id == "backingfiles:inventory-image:rescan" and .risk == "online"))
      and (.actions | any(.id == "mdraids:existing:assemble" and .risk == "offline-required"))
      and (.actions | any(.id == "mdraids:oldroot:stop" and .risk == "offline-required"))
      and (.actions | any(.id == "mdRaids:root:add-device:/dev/disk/by-id/nvme-md-spare" and .risk == "online"))
      and (.actions | any(.id == "multipathMaps:mpatha:add-device:/dev/sdb" and .risk == "online"))
      and (.actions | any(.id == "multipathmaps:mpathb:rescan" and .risk == "online"))
      and (.actions | any(.id == "multipathmaps:mpathold:destroy" and .risk == "offline-required"))
      and (.actions | any(.id == "partitions:root:grow" and .risk == "offline-required"))
      and (.actions | any(.id == "partitions:data-table:rescan" and .risk == "online"))
      and (.actions | any(.id == "swaps:primary:format" and .risk == "destructive"))
      and (.actions | any(.id == "swaps:inventory:rescan" and .risk == "online"))
      and (.actions | any(.id == "swaps:retired:deactivate" and .risk == "offline-required"))
      and (.actions | any(.id == "swaps:remove:destroy" and .risk == "destructive"))
      and (.actions | any(.id == "luks.devices:cryptroot:grow" and .risk == "offline-required"))
      and (.actions | any(.id == "luks.devices:cryptarchive:open" and .risk == "offline-required"))
      and (.actions | any(.id == "luks.devices:cryptclosed:close" and .risk == "offline-required"))
      and (.actions | any(.id == "nvmenamespaces:/dev/nvme0:create" and .risk == "destructive"))
      and (.actions | any(.id == "nvmenamespaces:/dev/nvme1:rescan" and .risk == "online"))
      and (.actions | any(.id == "pools:vault:import" and .risk == "offline-required" and .context.readOnly == true))
      and (.actions | any(.id == "pools:moveme:export" and .risk == "offline-required"))
      and (.actions | any(.id == "volumegroups:importvg:import" and .risk == "offline-required"))
      and (.actions | any(.id == "volumegroups:exportvg:export" and .risk == "offline-required"))
      and (.actions | any(.id == "volumegroups:activevg:activate" and .risk == "offline-required"))
      and (.actions | any(.id == "datasets:tank/home:create" and .risk == "online"))
      and (.actions | any(.id == "datasets:tank/inventory:rescan" and .risk == "online"))
      and (.actions | any(.id == "datasets:tank/home-review:promote" and .risk == "offline-required"))
      and (.actions | any(.id == "datasets:tank/legacy:rename" and .risk == "offline-required"))
      and (.actions | any(.id == "datasets:tank/archive:destroy"))
      and (.actions | any(.id == "snapshot:tank/home@before-prune:rename:tank/home@retained" and .risk == "offline-required"))
      and (.actions | any(.id == "snapshot:/mnt/persist/@home-before-clone:clone:/mnt/persist/@home-review" and .risk == "reversible" and .context.readOnly == true))
      and (.actions | any(.id == "snapshot:tank/root@rollback-point:rollback"))
      and (.actions | any(.id == "snapshot:tank/home@inventory:rescan" and .risk == "online"))
      and (.actions | any(.id == "snapshot:/mnt/persist/@home-inventory:rescan" and .risk == "online"))
      and (.actions | any(.id == "exports:/srv/share:export" and .risk == "online"))
      and (.actions | any(.id == "exports:/srv/inventory:rescan" and .risk == "online"))
      and (.actions | any(.id == "exports:/srv/old-share:unexport" and .risk == "offline-required"))
      and (.actions | any(.id == "nfs.mounts:/srv/shared:mount" and .risk == "online"))
      and (.actions | any(.id == "nfs.mounts:/srv/inventory:rescan" and .risk == "online"))
      and (.actions | any(.id == "nfs.mounts:/srv/tuned:remount" and .risk == "online"))
      and (.actions | any(.id == "nfs.mounts:/srv/old:unmount" and .risk == "offline-required"))
      and (.actions | any(.id == "caches:/dev/bcache0:add-device:cache-set-uuid" and .risk == "online"))
      and (.actions | any(.id == "caches:/dev/bcache0:rescan" and .risk == "online"))
      and (.actions | any(.id == "caches:/dev/bcache0:set-property:bcache.cache-mode" and .risk == "safe"))
      and (.actions | any(.id == "caches:/dev/bcache0:set-property:bcache.set-journal-delay-ms" and .risk == "safe"))
      and (.actions | any(.id == "caches:tank/l2arc0:replace-device:/dev/disk/by-id/old-cache"))
    ' "$lifecyclePlan"

    ${diskNix}/bin/disk-nix apply --spec ${
      root + /examples/simple-root.json
    } --script-out "$scriptOut" --json > "$simpleApply"
    jq -e '
      .status == "dry-run"
      and .apply.blockedCount == 0
      and .commandSummary.commandCount == 2
      and .commandSummary.needsDesiredSizeCount == 0
      and .verificationSummary.stepCount == 1
    ' "$simpleApply"
    test -x "$scriptOut"
    grep -- 'xfs_growfs /' "$scriptOut"
    grep -- 'Post-apply verification commands' "$scriptOut"

    printf '%s\n' '{"spec":{},"apply":{}}' > "$emptySpec"
    ${diskNix}/bin/disk-nix apply --spec "$emptySpec" --execute --json > "$emptyExecute"
    jq -e '
      .status == "succeeded"
      and .apply.blockedCount == 0
      and .commandSummary.commandCount == 0
      and .verificationSummary.commandCount == 0
      and (.executionResults | length) == 0
    ' "$emptyExecute"

    failingToolDir="$TMPDIR/failing-tools"
    mkdir -p "$failingToolDir"
    cat > "$failingToolDir/truncate" <<'EOF'
    #!${pkgs.bash}/bin/bash
    echo "synthetic truncate failure for disk-nix report coverage" >&2
    exit 73
    EOF
    chmod +x "$failingToolDir/truncate"
    failingSpec="$TMPDIR/failing-apply.json"
    failingApply="$TMPDIR/failing-apply.out"
    failingApplyReport="$TMPDIR/failing-apply-report.json"
    failingApplyReceipt="$TMPDIR/failing-apply-receipt.json"
    jq -n --arg target "$TMPDIR/failing-backing.img" '{
      spec: {
        backingFiles: {
          ($target): {
            operation: "create",
            desiredSize: "1M"
          }
        }
      }
    }' > "$failingSpec"
    if PATH="$failingToolDir:${diskNix}/bin:$PATH" ${diskNix}/bin/disk-nix apply \
      --spec "$failingSpec" \
      --execute \
      --report-out "$failingApplyReport" \
      --receipt-out "$failingApplyReceipt" \
      --json > "$failingApply"; then
      echo "expected failing backing-file apply to fail" >&2
      exit 1
    fi
    jq -e --arg target "$TMPDIR/failing-backing.img" '
      .status == "failed"
      and .apply.blockedCount == 0
      and .commandSummary.commandCount == 3
      and (.executionResults | length) == 2
      and .executionResults[0].success == true
      and .executionResults[0].argv == ["test", "!", "-e", $target]
      and .executionResults[1].success == false
      and .executionResults[1].statusCode == 73
      and .executionResults[1].argv == ["truncate", "--size", "1M", $target]
      and (.executionResults[1].stderr | contains("synthetic truncate failure"))
      and .partialExecutionRecovery.failedPhase == "command"
      and .partialExecutionRecovery.failedCommand == ["truncate", "--size", "1M", $target]
      and .partialExecutionRecovery.completedMutatingCommandCount == 0
      and (.partialExecutionRecovery.retryReviewActionIds | length == 1)
      and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
      and (.messages[] | contains("execute failed: stopped after 2 command result(s)"))
      and (.recoveryActions | any(.kind == "review-execution-failure"))
      and (.recoveryActions | any(.kind == "inspect-current-state"))
      and (.recoveryActions | any(.kind == "preserve-recovery-points"))
    ' "$failingApply"
    cmp "$failingApply" "$failingApplyReport"
    jq -e --arg spec "$failingSpec" --arg target "$TMPDIR/failing-backing.img" '
      .receiptVersion == 1
      and .command == "apply"
      and .specPath == $spec
      and .executeRequested == true
      and .report.status == "failed"
      and .report.executionResults[1].argv == ["truncate", "--size", "1M", $target]
      and .report.partialExecutionRecovery.failedCommand == ["truncate", "--size", "1M", $target]
      and (.report.recoveryActions | any(.kind == "review-execution-failure"))
    ' "$failingApplyReceipt"

    rollbackToolDir="$TMPDIR/rollback-tools"
    mkdir -p "$rollbackToolDir"
    cat > "$rollbackToolDir/zfs" <<'EOF'
    #!${pkgs.bash}/bin/bash
    if [ "$1" = rollback ]; then
      echo "synthetic zfs rollback failure for disk-nix recovery coverage" >&2
      exit 74
    fi
    printf '{}\n'
    EOF
    chmod +x "$rollbackToolDir/zfs"
    rollbackSpec="$TMPDIR/failing-rollback.json"
    rollbackApply="$TMPDIR/failing-rollback.out"
    jq -n '{
      spec: {
        snapshots: {
          "tank/home@before": {
            rollback: true
          }
        }
      },
      apply: {
        allowPotentialDataLoss: true
      }
    }' > "$rollbackSpec"
    if PATH="$rollbackToolDir:${diskNix}/bin:$PATH" ${diskNix}/bin/disk-nix apply \
      --spec "$rollbackSpec" \
      --execute \
      --json > "$rollbackApply"; then
      echo "expected failing ZFS rollback apply to fail" >&2
      exit 1
    fi
    jq -e '
      .status == "failed"
      and .apply.blockedCount == 0
      and .commandSummary.commandCount == 2
      and (.executionResults | length) == 2
      and .executionResults[0].argv == ["zfs", "list", "-t", "snapshot", "-H", "-p", "tank/home@before"]
      and .executionResults[0].success == true
      and .executionResults[1].argv == ["zfs", "rollback", "tank/home@before"]
      and .executionResults[1].success == false
      and .executionResults[1].statusCode == 74
      and (.executionResults[1].stderr | contains("synthetic zfs rollback failure"))
      and .partialExecutionRecovery.failedPhase == "command"
      and .partialExecutionRecovery.failedCommand == ["zfs", "rollback", "tank/home@before"]
      and .partialExecutionRecovery.completedMutatingCommandCount == 0
      and (.partialExecutionRecovery.retryReviewActionIds | index("snapshot:tank/home@before:rollback") != null)
      and (.recoveryActions | any(
        .kind == "domain-recovery"
        and (.commands | any(.argv == ["zfs", "list", "-t", "snapshot", "-H", "-p", "tank/home@before"]))
        and (.commands | any(.argv == ["zfs", "list", "-H", "-p", "tank/home"]))
        and (.notes | any(contains("prefer cloning the snapshot")))
      ))
      and (.recoveryActions | any(
        .kind == "roll-forward-review"
        and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
        and (.commands | any(.argv == ["zfs", "list", "-t", "snapshot", "-H", "-p", "-o", "name,creation,used,referenced,userrefs", "-r", "tank/home"]))
      ))
      and (.recoveryActions | any(
        .kind == "rollback-review"
        and (.commands | all(.mutates == false))
        and (.commands | any(.argv == ["zfs", "list", "-t", "snapshot", "-H", "-p", "tank/home@before"]))
        and (.commands | any(.argv == ["zfs", "list", "-H", "-p", "tank/home"]))
      ))
      and (.recoveryActions | any(.kind == "preserve-recovery-points"))
    ' "$rollbackApply"

    if ${diskNix}/bin/disk-nix apply --spec ${
      root + /examples/lifecycle-update.json
    } --report-out "$lifecycleApplyReport" --json > "$lifecycleApply"; then
      echo "expected lifecycle example apply to be blocked" >&2
      exit 1
    fi
    jq -e '
      .status == "blocked"
      and .apply.blockedCount == 39
      and .apply.blockedSummary.offlineRequiredCount == 31
      and .apply.blockedSummary.destructiveCount == 4
      and .apply.blockedSummary.potentialDataLossCount == 4
      and .apply.blockedSummary.unsupportedCount == 0
      and (.apply.blocked | any(.id == "datasets:tank/legacy:rename"))
      and (.apply.blocked | any(.id == "datasets:tank/home-review:promote"))
      and (.apply.blocked | any(.id == "pools:vault:import"))
      and (.apply.blocked | any(.id == "btrfssubvolumes:/mnt/persist/@old-name:rename"))
      and (.apply.blocked | any(.id == "pools:moveme:export"))
      and (.apply.blocked | any(.id == "volumegroups:importvg:import"))
      and (.apply.blocked | any(.id == "volumegroups:exportvg:export"))
      and (.apply.blocked | any(.id == "volumegroups:activevg:activate"))
      and (.apply.blocked | any(.id == "iscsisessions:iqn.2026-06.example:storage.logout:logout"))
      and (.apply.blocked | any(.id == "luns:iqn.2026-06.example:storage/old:3:detach"))
      and (.apply.blocked | any(.id == "exports:/srv/old-share:unexport"))
      and (.apply.blocked | any(.id == "nfs.mounts:/srv/old:unmount"))
      and (.apply.blocked | any(.id == "volumes:vg0/archive:deactivate"))
      and (.apply.blocked | any(.id == "swaps:retired:deactivate"))
      and (.apply.blocked | any(.id == "swaps:remove:destroy"))
      and (.apply.blocked | any(.id == "vdovolumes:warmarchive:start"))
      and (.apply.blocked | any(.id == "vdovolumes:coldarchive:stop"))
      and (.apply.blocked | any(.id == "luks.devices:cryptarchive:open"))
      and (.apply.blocked | any(.id == "luks.devices:cryptclosed:close"))
      and (.apply.blocked | any(.id == "lukskeyslots:cryptroot:2:remove-key"))
      and (.apply.blocked | any(.id == "lukstokens:cryptroot:1:remove-token"))
      and (.apply.blocked | any(.id == "mdraids:existing:assemble"))
      and (.apply.blocked | any(.id == "mdraids:oldroot:stop"))
      and (.apply.blocked | any(.id == "multipathmaps:mpathold:destroy"))
      and (.apply.blocked | any(.id == "snapshot:tank/home@before-prune:rename:tank/home@retained"))
    ' "$lifecycleApply"
    jq -e '
      .status == "blocked"
      and .apply.blockedCount == 39
    ' "$lifecycleApplyReport"

    ${diskNix}/bin/disk-nix validate --spec ${
      root + /examples/lifecycle-update.json
    } --report-out "$lifecycleValidateReport" --json > "$lifecycleValidate"
    jq -e '
      .status == "blocked"
      and .apply.blockedCount == 39
      and .messages[0] == "apply policy blocked 39 action(s)"
    ' "$lifecycleValidate"
    cmp "$lifecycleValidate" "$lifecycleValidateReport"

    touch "$out"
  '';
}
