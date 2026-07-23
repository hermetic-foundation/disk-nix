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
}
