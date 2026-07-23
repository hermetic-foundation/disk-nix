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
        failureRecoverySources="${root + /scripts/integration-failure-recovery-smoke.sh} ${
          root + /scripts/integration-failure-recovery-smoke.d
        }/*.sh"
        ${pkgs.bash}/bin/bash -n ${root + /scripts/integration-failure-recovery-smoke.sh}
        for scenarioChunk in ${root + /scripts/integration-failure-recovery-smoke.d}/*.sh; do
          ${pkgs.bash}/bin/bash -n "$scenarioChunk"
        done
        ${pkgs.gnugrep}/bin/grep -q DISK_NIX_INTEGRATION_DESTRUCTIVE $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake_tools/lvs' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-grow-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-xfs-grow-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-btrfs-scrub-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-btrfs-rebalance-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-filesystem-trim-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-filesystem-check-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-filesystem-repair-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-swap-label-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-dm-rename-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-zfs-dataset-rename-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-btrfs-snapshot-clone-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-zfs-snapshot-clone-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-vg-rename-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-rollback-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-create-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-grow-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-attach-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-detach-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-nvme-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-attach-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-detach-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-destroy-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-grow-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-property-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-lio-rescan-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-tgt-attach-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-tgt-detach-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-tgt-destroy-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-tgt-grow-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-tgt-property-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-tgt-rescan-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-multipath-replace-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-md-add-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-iscsi-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-iscsi-login-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-format-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-close-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-grow-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-keyslot-add-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-token-import-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-keyslot-remove-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-token-remove-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-multipath-resize-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-attach-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-detach-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-replace-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM cache replacement failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:vg0/root:replace-device:vg0/root-cache' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-rescan-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-nfs-unmount-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-nfs-export-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-nfs-unexport-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-property-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-bcache-replace-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic bcache replacement failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'caches:/dev/bcache0:replace-device:/dev/disk/by-id/old-cache' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-bcache-rescan-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-cache-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q partialExecutionRecovery $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic resize failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM grow failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-thin-create-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM thin-pool create failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'thinpools:vg0/newpool:create' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-thin-grow-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM thin-pool grow failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'thinpools:vg0/thinpool:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic XFS grow failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic Btrfs scrub failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic Btrfs rebalance failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-btrfs-replace-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic Btrfs device replacement failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-bcachefs-replace-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic bcachefs replacement rereplicate failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:bulk:replace-device:/dev/disk/by-id/old-bcachefs-device' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic filesystem trim failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic filesystem check failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic filesystem repair failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-filesystem-property-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic filesystem property failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:scratch:set-property:label' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic swap label failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-zram-rescan-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic zram rescan failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'zram:rescan' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-zram-property-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic zram property inventory failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'zram:set-property:algorithm' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-loop-rescan-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic loop rescan failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'loopdevices:/dev/loop7:rescan' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-backing-file-rescan-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic backing-file rescan stat failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'backingfiles:inventory:rescan' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-backing-file-grow-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic backing-file grow truncate failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'backingfiles:root:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-backing-file-create-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic backing-file create truncate failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'backingfiles:new:create' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic device-mapper rename failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic ZFS dataset rename failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic Btrfs snapshot clone failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic ZFS snapshot clone failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM VG rename failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-lvm-vg-replace-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LVM VG replacement pvmove failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-zfs-pool-replace-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic ZFS pool replacement failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'pools:tank:replace-device:/dev/disk/by-id/old-zfs-vdev' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic zfs rollback failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace create failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace grow rescan failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace attach failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace detach failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic nvme namespace delete failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO create failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO attach ACL failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO detach unmap failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO destroy backstore failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'target-side LUN LIO native grow with backing capacity and host verification' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO property failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN LIO rescan inventory failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt create failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt attach bind failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt detach logicalunit failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt destroy target failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'target-side LUN tgt native grow with backing capacity and host verification' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt property failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic target-side LUN tgt rescan inventory failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-scst-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic SCST target-side LUN add_lun failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:scst.root:create' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'run_scst_failure_case' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-target-lun-scst-$name-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:scst.root:attach' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:scst.root:detach' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:scst.root:destroy' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:scst.root:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetLuns:iqn.2026-06.example:scst.root:set-property:read_only' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:scst.root:rescan' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q '"--mode", "logicalunit", "--op", "update"' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-host-lun-rescan-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic host-side LUN SCSI rescan failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'luns:iqn.2026-06.example:storage/root:0:rescan' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'run_multipath_failure_case' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic multipath add failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'multipathMaps:root-map:add-device:/dev/sdb' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic multipath remove failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'multipathMaps:root-map:remove-device:/dev/sde' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic multipath destroy flush failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'multipathmaps:root-map:destroy' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic multipath resize failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic multipath replace delete failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-md-create-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID create failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'mdraids:newroot:create' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-md-assemble-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID assemble failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'mdraids:existing:assemble' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-md-stop-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID stop failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'mdraids:oldroot:stop' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-md-grow-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID grow failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'mdraids:root:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID add-member failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-md-remove-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID remove-member failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic MD RAID replace failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS open failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS format failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS close failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS grow failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS keyslot add failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS token import failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS keyslot remove failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS token remove failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-luks-property-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic LUKS property failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'luks.devices:cryptroot:set-property:label' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic partition grow failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic NFS remount failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic NFS unmount failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic NFS export failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic NFS unexport failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'exports:share:export' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'exports:oldshare:unexport' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic iscsi logout failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic iscsi login failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-iscsi-rescan-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic iscsi rescan failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'iscsisessions:iqn.2026-06.example:storage.root:rescan' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic lvm cache attach failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic lvm cache detach failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic lvm cache rescan failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-create-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO create failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:new-cache:create' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-rescan-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO rescan stats failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:refresharchive:rescan' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO grow failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-physical-grow-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO physical grow failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:archive-physical:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-start-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO start failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:warmarchive:start' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-stop-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO stop failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:coldarchive:stop' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'fake-vdo-remove-tools' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO remove failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:old-cache:destroy' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic VDO property failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic bcache property failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic bcache rescan failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'synthetic lvm cache property failure' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'snapshot:tank/home@before:rollback' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme0:create' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme1:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme2:attach' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme3:detach' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'nvmenamespaces:/dev/nvme4:destroy' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:storage.root:create' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:storage.root:attach' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:storage.root:detach' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:storage.root:destroy' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:storage.root:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:tgt.root:create' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:tgt.root:attach' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:tgt.root:detach' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:tgt.root:destroy' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'targetluns:iqn.2026-06.example:tgt.root:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:data:replace-device:/dev/disk/by-id/old-btrfs-device' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'multipathmaps:root-map:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'multipathMaps:root-map:replace-device:/dev/sdc' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'mdRaids:root:add-device:/dev/disk/by-id/nvme-spare' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'mdRaids:root:replace-device:/dev/disk/by-id/old-md-member' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'volumeGroups:vg0:replace-device:/dev/disk/by-id/old-pv' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'luks.devices:cryptarchive:open' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'luks.devices:cryptnew:format' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'luks.devices:cryptclosed:close' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'luks.devices:cryptroot:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'lukskeyslots:cryptroot:1:add-key' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'lukstokens:cryptroot:0:import-token' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'lukskeyslots:rootremove:remove-key' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'lukstokens:rootremove:remove-token' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'partitions:root:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'nfs.mounts:/srv/tuned:remount' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'nfs.mounts:/srv/old:unmount' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'iscsisessions:iqn.2026-06.example:storage.old:logout' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'iscsisessions:iqn.2026-06.example:storage.root:login' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:vg0/root:add-device:vg0/root-cache' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:vg0/root:remove-device:vg0/root-cache' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'vdovolumes:archive:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'vdoVolumes:archive:set-property:writePolicy' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'caches:writeback-cache:set-property:bcache.cache-mode' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'lvmCaches:vg0/root:set-property:lvm.cache-mode' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'completedMutatingCommandCount' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'volumes:root:grow' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'xfs_growfs' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:data:scrub' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:data:rebalance' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:scratch:trim' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:home:check' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'filesystems:data:repair' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'swaps:primary:set-property:label' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'dmmaps:cryptswap:rename' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'datasets:tank/home:rename' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'snapshot:before-clone:clone:tank/home-review' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'volumegroups:vg-old:rename' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'roll-forward-review' $failureRecoverySources
        ${pkgs.gnugrep}/bin/grep -q 'rollback-review' $failureRecoverySources
        touch "$out"
      '';
}
