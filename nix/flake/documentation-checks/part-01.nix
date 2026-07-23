{
  pkgs,
  self,
  root,
  diskNix,
  integrationDiskoExamples,
  ...
}:

''
  ${pkgs.nodejs}/bin/node ${root + /scripts/check-docs-legibility.mjs} ${self}
  ${pkgs.nodejs}/bin/node --check ${root + /scripts/render-docs.mjs}
  ${pkgs.nodejs}/bin/node --check ${root + /scripts/check-docs-legibility.mjs}
  checklist=${root + /docs/developer/feature-checklist.md}
  checklistDetails=${root + /docs/developer/testing-proof-checklist.md}
  checklistSources="$checklist $checklistDetails"
  execSourcesArray=(
    ${root + /crates/disk-nix-exec/src/lib.rs}
    ${root + /crates/disk-nix-exec/src/tests.rs}
    ${root + /crates/disk-nix-exec/src/tests}/*.rs
    ${root + /crates/disk-nix-exec/src/sections}/*.rs
    ${root + /crates/disk-nix-exec/src/sections/action_commands}/*.rs
    ${root + /crates/disk-nix-exec/src/sections/block_device_commands}/*.rs
    ${root + /crates/disk-nix-exec/src/sections/cache_network_commands}/*.rs
    ${root + /crates/disk-nix-exec/src/sections/filesystem_commands}/*.rs
    ${root + /crates/disk-nix-exec/src/sections/recovery_domain_targets}/*.rs
    ${root + /crates/disk-nix-exec/src/sections/rollback_recipes}/*.rs
    ${root + /crates/disk-nix-exec/src/sections/target_lun_commands}/*.rs
    ${root + /crates/disk-nix-exec/src/sections/verification_commands}/*.rs
  )
  planSourcesArray=(
    ${root + /crates/disk-nix-plan/src/lib.rs}
    ${root + /crates/disk-nix-plan/src/tests.rs}
    ${root + /crates/disk-nix-plan/src/tests}/*.rs
    ${root + /crates/disk-nix-plan/src/sections}/*.rs
    ${root + /crates/disk-nix-plan/src/sections/action_builders}/*.rs
    ${root + /crates/disk-nix-plan/src/sections/action_fields}/*.rs
    ${root + /crates/disk-nix-plan/src/sections/capabilities}/*.rs
    ${root + /crates/disk-nix-plan/src/sections/dependencies}/*.rs
    ${root + /crates/disk-nix-plan/src/sections/local_diagnostics}/*.rs
    ${root + /crates/disk-nix-plan/src/sections/mapping_diagnostics}/*.rs
    ${root + /crates/disk-nix-plan/src/sections/operation_classification}/*.rs
    ${root + /crates/disk-nix-plan/src/sections/storage_diagnostics}/*.rs
    ${root + /crates/disk-nix-plan/src/sections/topology_properties}/*.rs
  )
  cliSourcesArray=(
    ${root + /crates/disk-nix-cli/src/main.rs}
    ${root + /crates/disk-nix-cli/src/tests.rs}
    ${root + /crates/disk-nix-cli/src/tests}/*.rs
    ${root + /crates/disk-nix-cli/src/tests/part_03}/*.rs
    ${root + /crates/disk-nix-cli/src/sections}/*.rs
    ${root + /crates/disk-nix-cli/src/sections/usage_details}/*.rs
  )
  failureRecoverySourcesArray=(
    ${root + /scripts/integration-failure-recovery-smoke.sh}
    ${root + /scripts/integration-failure-recovery-smoke.d}/*.sh
  )
  execSources="$(printf '%s\n' "''${execSourcesArray[@]}")"
  planSources="$(printf '%s\n' "''${planSourcesArray[@]}")"
  cliSources="$(printf '%s\n' "''${cliSourcesArray[@]}")"
  failureRecoverySources="$(printf '%s\n' "''${failureRecoverySourcesArray[@]}")"
  ${pkgs.gnugrep}/bin/grep -q 'docs/index.md' ${root + /README.md}
  ${pkgs.gnugrep}/bin/grep -q 'docs/user/user-guide.md' ${root + /README.md}
  ${pkgs.gnugrep}/bin/grep -q 'docs/developer/feature-checklist.md' ${root + /README.md}
  ${pkgs.gnugrep}/bin/grep -q 'docs/developer/testing-proof-checklist.md' ${root + /README.md}
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
  ${pkgs.gnugrep}/bin/grep -q 'Status labels:' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'Update rules:' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q '\*\*Finished:\*\*' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q '`Partial`: useful support exists' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q '`Desired`: not implemented yet' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'Operator runbooks for high-risk workflows' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'multi-domain mutation' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'VM-backed failure' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'fresh-topology review' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback-review behavior' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'operator-only guidance instead of automated unsafe rollback' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'MD RAID degraded' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real MD RAID member' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'mdadm <array> --replace <old-loop> --with <new-loop>' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'MD RAID stale-superblock' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'MD RAID failed-detach' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'MD RAID failed-reattach' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'MD RAID partial-rebuild' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'replacement-race coverage' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'missing-member coverage: the loop-backed MD harness' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'layered block/filesystem' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'LVM cache data-survival' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'LVM cache, and multipath-backed stacks' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'replacement data-survival coverage' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'cache-device failure-state coverage' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real bcache read-only' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real bcache cache detach/reattach' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'rescan coverage: the loop-backed bcache harness' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'network-storage scenarios' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'NFS failed-and-resumed remount data-survival' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'iSCSI host-LUN failed-and-resumed rescan data-survival' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'target-side LUN failed-and-resumed' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'lab-backed NVMe namespace create/delete' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'lab-backed NVMe namespace grow' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'lab-backed NVMe namespace attach/detach' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'NVMe namespace identity-drift assertions' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'lab-backed multipath' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'path replacement coverage' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real filesystem' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real LUKS header' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real Btrfs filesystem' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real swap signature' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real ZFS pool' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real ZFS pool device replacement' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real bcachefs member replacement' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real LVM cache' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real bcache property' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real bcache cache replacement' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real loop-device' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real backing-file' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real zram property' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real target-side LUN' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'LIO target-side' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'map/unmap coverage: the loop-backed target LUN harness' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'destroy refusal coverage' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real VDO volume' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'real NFS export' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'e2label' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'cryptsetup config' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'btrfs filesystem label' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'swaplabel' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'zpool set' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'lvchange --cachemode' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'disk-nix-bcache-property' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'blockdev --setro' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'chmod 0600' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'zramctl --bytes --raw --noheadings --output-all' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'emulate_write_cache=0' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'vdo changeWritePolicy' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'exportfs -i' $checklistSources
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
''
