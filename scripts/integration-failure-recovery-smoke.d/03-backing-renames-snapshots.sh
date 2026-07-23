backing_file_rescan_tools="$tmpdir/fake-backing-file-rescan-tools"
mkdir -p "$backing_file_rescan_tools"
backing_file_rescan_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$backing_file_rescan_tools/stat" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--printf=%n %s %b %B\\n /var/lib/images/inventory.img" ]]; then
  echo "synthetic backing-file rescan stat failure for disk-nix recovery coverage" >&2
  exit 87
fi
printf '/var/lib/images/inventory.img 1048576 8 512\n'
EOF

cat > "$backing_file_rescan_tools/du" <<'EOF'
#!/usr/bin/env bash
printf '1048576\t/var/lib/images/inventory.img\n'
EOF

cat > "$backing_file_rescan_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$backing_file_rescan_disk_nix" "\$@"
EOF

chmod +x "$backing_file_rescan_tools/stat" "$backing_file_rescan_tools/du" "$backing_file_rescan_tools/disk-nix"

backing_file_rescan_spec="$tmpdir/backing-file-rescan-spec.json"
backing_file_rescan_json="$tmpdir/backing-file-rescan-apply.json"
backing_file_rescan_report="$tmpdir/backing-file-rescan-report.json"
backing_file_rescan_receipt="$tmpdir/backing-file-rescan-receipt.json"

jq -n '{
  backingFiles: {
    inventory: {
      operation: "rescan",
      path: "/var/lib/images/inventory.img"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$backing_file_rescan_spec"

if PATH="$backing_file_rescan_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$backing_file_rescan_spec" \
  --execute \
  --report-out "$backing_file_rescan_report" \
  --receipt-out "$backing_file_rescan_receipt" \
  --json > "$backing_file_rescan_json"; then
  echo "expected synthetic backing-file rescan failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 3
  and .commandSummary.mutatingCount == 0
  and (.executionResults | length) == 1
  and .executionResults[0].success == false
  and .executionResults[0].statusCode == 87
  and .executionResults[0].actionId == "backingfiles:inventory:rescan"
  and .executionResults[0].argv == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/inventory.img"]
  and (.executionResults[0].stderr | contains("synthetic backing-file rescan stat failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "backingfiles:inventory:rescan"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/inventory.img"]
  and .partialExecutionRecovery.retryReviewActionIds == ["backingfiles:inventory:rescan"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/inventory.img"]))
    and (.commands | any(.argv == ["du", "--bytes", "--apparent-size", "/var/lib/images/inventory.img"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/var/lib/images/inventory.img", "--json"]))
    and (.notes | any(contains("local mapping changes")))
    and (.notes | any(contains("backing file size")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/inventory.img"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/var/lib/images/inventory.img", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/inventory.img"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/var/lib/images/inventory.img", "--json"]))
  ))
' "$backing_file_rescan_json" >/dev/null

cmp "$backing_file_rescan_json" "$backing_file_rescan_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "backingfiles:inventory:rescan"
  and .report.partialExecutionRecovery.failedCommand == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/inventory.img"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$backing_file_rescan_receipt" >/dev/null

backing_file_grow_tools="$tmpdir/fake-backing-file-grow-tools"
mkdir -p "$backing_file_grow_tools"
backing_file_grow_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$backing_file_grow_tools/stat" <<'EOF'
#!/usr/bin/env bash
printf '/var/lib/images/root.img 1048576 8 512\n'
EOF

cat > "$backing_file_grow_tools/du" <<'EOF'
#!/usr/bin/env bash
printf '1048576\t/var/lib/images/root.img\n'
EOF

cat > "$backing_file_grow_tools/truncate" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--size 16GiB /var/lib/images/root.img" ]]; then
  echo "synthetic backing-file grow truncate failure for disk-nix recovery coverage" >&2
  exit 88
fi
printf '{}\n'
EOF

cat > "$backing_file_grow_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$backing_file_grow_disk_nix" "\$@"
EOF

chmod +x "$backing_file_grow_tools/stat" "$backing_file_grow_tools/du" "$backing_file_grow_tools/truncate" "$backing_file_grow_tools/disk-nix"

backing_file_grow_spec="$tmpdir/backing-file-grow-spec.json"
backing_file_grow_json="$tmpdir/backing-file-grow-apply.json"
backing_file_grow_report="$tmpdir/backing-file-grow-report.json"
backing_file_grow_receipt="$tmpdir/backing-file-grow-receipt.json"

jq -n '{
  backingFiles: {
    root: {
      operation: "grow",
      path: "/var/lib/images/root.img",
      desiredSize: "16GiB"
    }
  },
  apply: {
    allowGrow: true,
    allowOffline: true
  }
}' > "$backing_file_grow_spec"

if PATH="$backing_file_grow_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$backing_file_grow_spec" \
  --execute \
  --report-out "$backing_file_grow_report" \
  --receipt-out "$backing_file_grow_receipt" \
  --json > "$backing_file_grow_json"; then
  echo "expected synthetic backing-file grow failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and .commandSummary.mutatingCount == 1
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "backingfiles:root:grow"
  and .executionResults[0].argv == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/root.img"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 88
  and .executionResults[1].actionId == "backingfiles:root:grow"
  and .executionResults[1].argv == ["truncate", "--size", "16GiB", "/var/lib/images/root.img"]
  and (.executionResults[1].stderr | contains("synthetic backing-file grow truncate failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "backingfiles:root:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["truncate", "--size", "16GiB", "/var/lib/images/root.img"]
  and .partialExecutionRecovery.retryReviewActionIds == ["backingfiles:root:grow"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/root.img"]))
    and (.commands | any(.argv == ["du", "--bytes", "--apparent-size", "/var/lib/images/root.img"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/var/lib/images/root.img", "--json"]))
    and (.notes | any(contains("backing file size")))
    and (.notes | any(contains("local mapping changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/root.img"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/var/lib/images/root.img", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/root.img"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/var/lib/images/root.img", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$backing_file_grow_json" >/dev/null

cmp "$backing_file_grow_json" "$backing_file_grow_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "backingfiles:root:grow"
  and .report.partialExecutionRecovery.failedCommand == ["truncate", "--size", "16GiB", "/var/lib/images/root.img"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$backing_file_grow_receipt" >/dev/null

backing_file_create_tools="$tmpdir/fake-backing-file-create-tools"
mkdir -p "$backing_file_create_tools"
backing_file_create_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$backing_file_create_tools/truncate" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--size 8GiB /var/lib/images/new.img" ]]; then
  echo "synthetic backing-file create truncate failure for disk-nix recovery coverage" >&2
  exit 89
fi
printf '{}\n'
EOF

cat > "$backing_file_create_tools/stat" <<'EOF'
#!/usr/bin/env bash
printf '/var/lib/images/new.img 0 0 512\n'
EOF

cat > "$backing_file_create_tools/du" <<'EOF'
#!/usr/bin/env bash
printf '0\t/var/lib/images/new.img\n'
EOF

cat > "$backing_file_create_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$backing_file_create_disk_nix" "\$@"
EOF

chmod +x "$backing_file_create_tools/truncate" "$backing_file_create_tools/stat" "$backing_file_create_tools/du" "$backing_file_create_tools/disk-nix"

backing_file_create_spec="$tmpdir/backing-file-create-spec.json"
backing_file_create_json="$tmpdir/backing-file-create-apply.json"
backing_file_create_report="$tmpdir/backing-file-create-report.json"
backing_file_create_receipt="$tmpdir/backing-file-create-receipt.json"

jq -n '{
  backingFiles: {
    new: {
      operation: "create",
      path: "/var/lib/images/new.img",
      desiredSize: "8GiB"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$backing_file_create_spec"

if PATH="$backing_file_create_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$backing_file_create_spec" \
  --execute \
  --report-out "$backing_file_create_report" \
  --receipt-out "$backing_file_create_receipt" \
  --json > "$backing_file_create_json"; then
  echo "expected synthetic backing-file create failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 3
  and .commandSummary.mutatingCount == 1
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "backingfiles:new:create"
  and .executionResults[0].argv == ["test", "!", "-e", "/var/lib/images/new.img"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 89
  and .executionResults[1].actionId == "backingfiles:new:create"
  and .executionResults[1].argv == ["truncate", "--size", "8GiB", "/var/lib/images/new.img"]
  and (.executionResults[1].stderr | contains("synthetic backing-file create truncate failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "backingfiles:new:create"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["truncate", "--size", "8GiB", "/var/lib/images/new.img"]
  and .partialExecutionRecovery.retryReviewActionIds == ["backingfiles:new:create"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/new.img"]))
    and (.commands | any(.argv == ["du", "--bytes", "--apparent-size", "/var/lib/images/new.img"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/var/lib/images/new.img", "--json"]))
    and (.notes | any(contains("backing file size")))
    and (.notes | any(contains("local mapping changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/new.img"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/var/lib/images/new.img", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["stat", "--printf=%n %s %b %B\\n", "/var/lib/images/new.img"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/var/lib/images/new.img", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$backing_file_create_json" >/dev/null

cmp "$backing_file_create_json" "$backing_file_create_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "backingfiles:new:create"
  and .report.partialExecutionRecovery.failedCommand == ["truncate", "--size", "8GiB", "/var/lib/images/new.img"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$backing_file_create_receipt" >/dev/null

dm_rename_tools="$tmpdir/fake-dm-rename-tools"
mkdir -p "$dm_rename_tools"

cat > "$dm_rename_tools/dmsetup" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "rename /dev/mapper/cryptswap cryptswap-retired" ]]; then
  echo "synthetic device-mapper rename failure for disk-nix recovery coverage" >&2
  exit 76
fi
printf '{}\n'
EOF

chmod +x "$dm_rename_tools/dmsetup"

dm_rename_spec="$tmpdir/dm-rename-spec.json"
dm_rename_json="$tmpdir/dm-rename-apply.json"
dm_rename_report="$tmpdir/dm-rename-report.json"
dm_rename_receipt="$tmpdir/dm-rename-receipt.json"

jq -n '{
  dmMaps: {
    cryptswap: {
      operation: "rename",
      target: "/dev/mapper/cryptswap",
      renameTo: "/dev/mapper/cryptswap-retired"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$dm_rename_spec"

if PATH="$dm_rename_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$dm_rename_spec" \
  --execute \
  --report-out "$dm_rename_report" \
  --receipt-out "$dm_rename_receipt" \
  --json > "$dm_rename_json"; then
  echo "expected synthetic device-mapper rename failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 4
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["dmsetup", "info", "-c", "--noheadings", "-o", "name,uuid,major,minor,open,segments,events", "/dev/mapper/cryptswap"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["dmsetup", "deps", "-o", "devname", "/dev/mapper/cryptswap"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 76
  and .executionResults[2].argv == ["dmsetup", "rename", "/dev/mapper/cryptswap", "cryptswap-retired"]
  and (.executionResults[2].stderr | contains("synthetic device-mapper rename failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "dmmaps:cryptswap:rename"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["dmsetup", "rename", "/dev/mapper/cryptswap", "cryptswap-retired"]
  and .partialExecutionRecovery.retryReviewActionIds == ["dmmaps:cryptswap:rename"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["dmsetup", "info", "-c", "--noheadings", "-o", "name,uuid,major,minor,open,segments,events", "/dev/mapper/cryptswap"]))
    and (.commands | any(.argv == ["dmsetup", "deps", "-o", "devname", "/dev/mapper/cryptswap"]))
    and (.commands | any(.argv == ["dmsetup", "table", "/dev/mapper/cryptswap"]))
    and (.commands | any(.argv == ["dmsetup", "status", "/dev/mapper/cryptswap"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/mapper/cryptswap", "--json"]))
    and (.notes | any(contains("local mapping changes")))
    and (.notes | any(contains("dependencies")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["dmsetup", "status", "/dev/mapper/cryptswap"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["dmsetup", "table", "/dev/mapper/cryptswap"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$dm_rename_json" >/dev/null

cmp "$dm_rename_json" "$dm_rename_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "dmmaps:cryptswap:rename"
  and .report.partialExecutionRecovery.failedCommand == ["dmsetup", "rename", "/dev/mapper/cryptswap", "cryptswap-retired"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$dm_rename_receipt" >/dev/null

zfs_dataset_rename_tools="$tmpdir/fake-zfs-dataset-rename-tools"
mkdir -p "$zfs_dataset_rename_tools"

cat > "$zfs_dataset_rename_tools/zfs" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "rename tank/home tank/home-staged" ]]; then
  echo "synthetic ZFS dataset rename failure for disk-nix recovery coverage" >&2
  exit 77
fi
printf '{}\n'
EOF

chmod +x "$zfs_dataset_rename_tools/zfs"

zfs_dataset_rename_spec="$tmpdir/zfs-dataset-rename-spec.json"
zfs_dataset_rename_json="$tmpdir/zfs-dataset-rename-apply.json"
zfs_dataset_rename_report="$tmpdir/zfs-dataset-rename-report.json"
zfs_dataset_rename_receipt="$tmpdir/zfs-dataset-rename-receipt.json"

jq -n '{
  datasets: {
    "tank/home": {
      operation: "rename",
      renameTo: "tank/home-staged"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$zfs_dataset_rename_spec"

if PATH="$zfs_dataset_rename_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$zfs_dataset_rename_spec" \
  --execute \
  --report-out "$zfs_dataset_rename_report" \
  --receipt-out "$zfs_dataset_rename_receipt" \
  --json > "$zfs_dataset_rename_json"; then
  echo "expected synthetic ZFS dataset rename failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["zfs", "list", "-H", "-p", "tank/home"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 77
  and .executionResults[1].argv == ["zfs", "rename", "tank/home", "tank/home-staged"]
  and (.executionResults[1].stderr | contains("synthetic ZFS dataset rename failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "datasets:tank/home:rename"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["zfs", "rename", "tank/home", "tank/home-staged"]
  and .partialExecutionRecovery.retryReviewActionIds == ["datasets:tank/home:rename"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["zfs", "list", "-H", "-p", "-t", "filesystem", "tank/home"]))
    and (.commands | any(.argv == ["zfs", "get", "all", "tank/home"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "tank/home", "--json"]))
    and (.notes | any(contains("ZFS changes")))
    and (.notes | any(contains("LUN consumers")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["zfs", "get", "all", "tank/home"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["zfs", "list", "-H", "-p", "-t", "filesystem", "tank/home"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$zfs_dataset_rename_json" >/dev/null

cmp "$zfs_dataset_rename_json" "$zfs_dataset_rename_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "datasets:tank/home:rename"
  and .report.partialExecutionRecovery.failedCommand == ["zfs", "rename", "tank/home", "tank/home-staged"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$zfs_dataset_rename_receipt" >/dev/null

btrfs_snapshot_clone_tools="$tmpdir/fake-btrfs-snapshot-clone-tools"
mkdir -p "$btrfs_snapshot_clone_tools"

cat > "$btrfs_snapshot_clone_tools/btrfs" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "subvolume snapshot -r /mnt/persist/@home-before /mnt/persist/@home-review" ]]; then
  echo "synthetic Btrfs snapshot clone failure for disk-nix recovery coverage" >&2
  exit 78
fi
printf '{}\n'
EOF

chmod +x "$btrfs_snapshot_clone_tools/btrfs"

btrfs_snapshot_clone_spec="$tmpdir/btrfs-snapshot-clone-spec.json"
btrfs_snapshot_clone_json="$tmpdir/btrfs-snapshot-clone-apply.json"
btrfs_snapshot_clone_report="$tmpdir/btrfs-snapshot-clone-report.json"
btrfs_snapshot_clone_receipt="$tmpdir/btrfs-snapshot-clone-receipt.json"

jq -n '{
  snapshots: {
    "/mnt/persist/@home-before": {
      target: "/mnt/persist/@home",
      cloneTo: "/mnt/persist/@home-review",
      readOnly: true
    }
  }
}' > "$btrfs_snapshot_clone_spec"

if PATH="$btrfs_snapshot_clone_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$btrfs_snapshot_clone_spec" \
  --execute \
  --report-out "$btrfs_snapshot_clone_report" \
  --receipt-out "$btrfs_snapshot_clone_receipt" \
  --json > "$btrfs_snapshot_clone_json"; then
  echo "expected synthetic Btrfs snapshot clone failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["btrfs", "subvolume", "show", "/mnt/persist/@home-before"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 78
  and .executionResults[1].argv == ["btrfs", "subvolume", "snapshot", "-r", "/mnt/persist/@home-before", "/mnt/persist/@home-review"]
  and (.executionResults[1].stderr | contains("synthetic Btrfs snapshot clone failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["btrfs", "subvolume", "snapshot", "-r", "/mnt/persist/@home-before", "/mnt/persist/@home-review"]
  and .partialExecutionRecovery.retryReviewActionIds == ["snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["btrfs", "subvolume", "show", "/mnt/persist/@home-before"]))
    and (.commands | any(.argv == ["btrfs", "property", "get", "-ts", "/mnt/persist/@home-before", "ro"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/mnt/persist/@home-before", "--json"]))
    and (.notes | any(contains("snapshot lifecycle")))
    and (.notes | any(contains("read-only state")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["btrfs", "subvolume", "show", "/mnt/persist/@home-review"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["btrfs", "property", "get", "-ts", "/mnt/persist/@home-before", "ro"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$btrfs_snapshot_clone_json" >/dev/null

cmp "$btrfs_snapshot_clone_json" "$btrfs_snapshot_clone_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "snapshot:/mnt/persist/@home-before:clone:/mnt/persist/@home-review"
  and .report.partialExecutionRecovery.failedCommand == ["btrfs", "subvolume", "snapshot", "-r", "/mnt/persist/@home-before", "/mnt/persist/@home-review"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$btrfs_snapshot_clone_receipt" >/dev/null

zfs_snapshot_clone_tools="$tmpdir/fake-zfs-snapshot-clone-tools"
mkdir -p "$zfs_snapshot_clone_tools"

cat > "$zfs_snapshot_clone_tools/zfs" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "clone tank/home@before tank/home-review" ]]; then
  echo "synthetic ZFS snapshot clone failure for disk-nix recovery coverage" >&2
  exit 80
fi
printf '{}\n'
EOF

chmod +x "$zfs_snapshot_clone_tools/zfs"

zfs_snapshot_clone_spec="$tmpdir/zfs-snapshot-clone-spec.json"
zfs_snapshot_clone_json="$tmpdir/zfs-snapshot-clone-apply.json"
zfs_snapshot_clone_report="$tmpdir/zfs-snapshot-clone-report.json"
zfs_snapshot_clone_receipt="$tmpdir/zfs-snapshot-clone-receipt.json"

jq -n '{
  snapshots: {
    "before-clone": {
      name: "tank/home@before",
      target: "tank/home",
      cloneTo: "tank/home-review"
    }
  }
}' > "$zfs_snapshot_clone_spec"

if PATH="$zfs_snapshot_clone_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$zfs_snapshot_clone_spec" \
  --execute \
  --report-out "$zfs_snapshot_clone_report" \
  --receipt-out "$zfs_snapshot_clone_receipt" \
  --json > "$zfs_snapshot_clone_json"; then
  echo "expected synthetic ZFS snapshot clone failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["zfs", "list", "-t", "snapshot", "-H", "-p", "tank/home@before"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 80
  and .executionResults[1].argv == ["zfs", "clone", "tank/home@before", "tank/home-review"]
  and (.executionResults[1].stderr | contains("synthetic ZFS snapshot clone failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "snapshot:before-clone:clone:tank/home-review"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["zfs", "clone", "tank/home@before", "tank/home-review"]
  and .partialExecutionRecovery.retryReviewActionIds == ["snapshot:before-clone:clone:tank/home-review"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["zfs", "list", "-t", "snapshot", "-H", "-p", "tank/home@before"]))
    and (.commands | any(.argv == ["zfs", "holds", "tank/home@before"]))
    and (.commands | any(.argv == ["zfs", "list", "-t", "snapshot", "-H", "-p", "-o", "name,creation,used,referenced,userrefs", "-r", "tank/home"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "tank/home@before", "--json"]))
    and (.notes | any(contains("snapshot lifecycle")))
    and (.notes | any(contains("hold tags")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["zfs", "list", "-H", "-p", "tank/home-review"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["zfs", "holds", "tank/home@before"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$zfs_snapshot_clone_json" >/dev/null

cmp "$zfs_snapshot_clone_json" "$zfs_snapshot_clone_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "snapshot:before-clone:clone:tank/home-review"
  and .report.partialExecutionRecovery.failedCommand == ["zfs", "clone", "tank/home@before", "tank/home-review"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$zfs_snapshot_clone_receipt" >/dev/null

lvm_vg_rename_tools="$tmpdir/fake-lvm-vg-rename-tools"
mkdir -p "$lvm_vg_rename_tools"

cat > "$lvm_vg_rename_tools/vgs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$lvm_vg_rename_tools/pvs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$lvm_vg_rename_tools/lvs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$lvm_vg_rename_tools/vgrename" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "vg-old vg-new" ]]; then
  echo "synthetic LVM VG rename failure for disk-nix recovery coverage" >&2
  exit 79
fi
printf '{}\n'
EOF

chmod +x "$lvm_vg_rename_tools/vgs" "$lvm_vg_rename_tools/pvs" "$lvm_vg_rename_tools/lvs" "$lvm_vg_rename_tools/vgrename"

lvm_vg_rename_spec="$tmpdir/lvm-vg-rename-spec.json"
lvm_vg_rename_json="$tmpdir/lvm-vg-rename-apply.json"
lvm_vg_rename_report="$tmpdir/lvm-vg-rename-report.json"
lvm_vg_rename_receipt="$tmpdir/lvm-vg-rename-receipt.json"

jq -n '{
  volumeGroups: {
    "vg-old": {
      operation: "rename",
      renameTo: "vg-new"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$lvm_vg_rename_spec"

if PATH="$lvm_vg_rename_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$lvm_vg_rename_spec" \
  --execute \
  --report-out "$lvm_vg_rename_report" \
  --receipt-out "$lvm_vg_rename_receipt" \
  --json > "$lvm_vg_rename_json"; then
  echo "expected synthetic LVM VG rename failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["vgs", "--reportformat", "json", "vg-old"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 79
  and .executionResults[1].argv == ["vgrename", "vg-old", "vg-new"]
  and (.executionResults[1].stderr | contains("synthetic LVM VG rename failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "volumegroups:vg-old:rename"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["vgrename", "vg-old", "vg-new"]
  and .partialExecutionRecovery.retryReviewActionIds == ["volumegroups:vg-old:rename"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["vgs", "--reportformat", "json", "vg-old"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "vg-old", "--json"]))
    and (.notes | any(contains("LVM changes")))
    and (.notes | any(contains("import, export")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["vgs", "--reportformat", "json", "vg-old"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$lvm_vg_rename_json" >/dev/null

cmp "$lvm_vg_rename_json" "$lvm_vg_rename_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "volumegroups:vg-old:rename"
  and .report.partialExecutionRecovery.failedCommand == ["vgrename", "vg-old", "vg-new"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$lvm_vg_rename_receipt" >/dev/null

lvm_vg_replace_tools="$tmpdir/fake-lvm-vg-replace-tools"
mkdir -p "$lvm_vg_replace_tools"
lvm_vg_replace_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$lvm_vg_replace_tools/vgs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$lvm_vg_replace_tools/pvs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$lvm_vg_replace_tools/lvs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$lvm_vg_replace_tools/vgextend" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$lvm_vg_replace_tools/pvmove" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/dev/disk/by-id/old-pv /dev/disk/by-id/new-pv" ]]; then
  echo "synthetic LVM VG replacement pvmove failure for disk-nix recovery coverage" >&2
  exit 81
fi
printf '{}\n'
EOF

cat > "$lvm_vg_replace_tools/vgreduce" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$lvm_vg_replace_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$lvm_vg_replace_disk_nix" "\$@"
EOF

chmod +x "$lvm_vg_replace_tools/vgs" "$lvm_vg_replace_tools/pvs" "$lvm_vg_replace_tools/lvs" "$lvm_vg_replace_tools/vgextend" "$lvm_vg_replace_tools/pvmove" "$lvm_vg_replace_tools/vgreduce" "$lvm_vg_replace_tools/disk-nix"

lvm_vg_replace_spec="$tmpdir/lvm-vg-replace-spec.json"
lvm_vg_replace_json="$tmpdir/lvm-vg-replace-apply.json"
lvm_vg_replace_report="$tmpdir/lvm-vg-replace-report.json"
lvm_vg_replace_receipt="$tmpdir/lvm-vg-replace-receipt.json"

jq -n '{
  volumeGroups: {
    vg0: {
      target: "vg0",
      replaceDevices: {
        "/dev/disk/by-id/old-pv": "/dev/disk/by-id/new-pv"
      }
    }
  },
  apply: {
    allowOffline: true,
    allowDeviceReplacement: true
  }
}' > "$lvm_vg_replace_spec"

if PATH="$lvm_vg_replace_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$lvm_vg_replace_spec" \
  --execute \
  --report-out "$lvm_vg_replace_report" \
  --receipt-out "$lvm_vg_replace_receipt" \
  --json > "$lvm_vg_replace_json"; then
  echo "expected synthetic LVM VG replacement failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 5
  and (.executionResults | length) == 4
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["pvs", "--reportformat", "json", "/dev/disk/by-id/old-pv"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["pvs", "--reportformat", "json", "/dev/disk/by-id/new-pv"]
  and .executionResults[2].success == true
  and .executionResults[2].argv == ["vgextend", "vg0", "/dev/disk/by-id/new-pv"]
  and .executionResults[3].success == false
  and .executionResults[3].statusCode == 81
  and .executionResults[3].argv == ["pvmove", "/dev/disk/by-id/old-pv", "/dev/disk/by-id/new-pv"]
  and (.executionResults[3].stderr | contains("synthetic LVM VG replacement pvmove failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "volumeGroups:vg0:replace-device:/dev/disk/by-id/old-pv"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["pvmove", "/dev/disk/by-id/old-pv", "/dev/disk/by-id/new-pv"]
  and .partialExecutionRecovery.retryReviewActionIds == ["volumeGroups:vg0:replace-device:/dev/disk/by-id/old-pv"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["vgs", "--reportformat", "json", "/dev/disk/by-id/old-pv"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/old-pv", "--json"]))
    and (.notes | any(contains("LVM changes")))
    and (.notes | any(contains("pvmove")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["vgs", "--reportformat", "json", "/dev/disk/by-id/old-pv"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "vg0", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["vgs", "--reportformat", "json", "/dev/disk/by-id/old-pv"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$lvm_vg_replace_json" >/dev/null

cmp "$lvm_vg_replace_json" "$lvm_vg_replace_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "volumeGroups:vg0:replace-device:/dev/disk/by-id/old-pv"
  and .report.partialExecutionRecovery.failedCommand == ["pvmove", "/dev/disk/by-id/old-pv", "/dev/disk/by-id/new-pv"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$lvm_vg_replace_receipt" >/dev/null
