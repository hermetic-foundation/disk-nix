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
  execSourcesArray=(
    ${root + /crates/disk-nix-exec/src/lib.rs}
    ${root + /crates/disk-nix-exec/src/tests.rs}
    ${root + /crates/disk-nix-exec/src/tests}/*.rs
    ${root + /crates/disk-nix-exec/src/sections}/*.rs
    ${root + /crates/disk-nix-exec/src/sections/action_commands}/*.rs
    ${root + /crates/disk-nix-exec/src/sections/block_device_commands}/*.rs
    ${root + /crates/disk-nix-exec/src/sections/target_lun_commands}/*.rs
    ${root + /crates/disk-nix-exec/src/sections/verification_commands}/*.rs
  )
  planSourcesArray=(
    ${root + /crates/disk-nix-plan/src/lib.rs}
    ${root + /crates/disk-nix-plan/src/tests.rs}
    ${root + /crates/disk-nix-plan/src/tests}/*.rs
    ${root + /crates/disk-nix-plan/src/sections}/*.rs
    ${root + /crates/disk-nix-plan/src/sections/capabilities}/*.rs
    ${root + /crates/disk-nix-plan/src/sections/operation_classification}/*.rs
  )
  cliSourcesArray=(
    ${root + /crates/disk-nix-cli/src/main.rs}
    ${root + /crates/disk-nix-cli/src/tests.rs}
    ${root + /crates/disk-nix-cli/src/tests}/*.rs
    ${root + /crates/disk-nix-cli/src/sections}/*.rs
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
''
