filesystem_trim_tools="$tmpdir/fake-filesystem-trim-tools"
mkdir -p "$filesystem_trim_tools"

cat > "$filesystem_trim_tools/findmnt" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$filesystem_trim_tools/fstrim" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "-v /scratch" ]]; then
  echo "synthetic filesystem trim failure for disk-nix recovery coverage" >&2
  exit 83
fi
printf '{}\n'
EOF

chmod +x "$filesystem_trim_tools/findmnt" "$filesystem_trim_tools/fstrim"

filesystem_trim_spec="$tmpdir/filesystem-trim-spec.json"
filesystem_trim_json="$tmpdir/filesystem-trim-apply.json"
filesystem_trim_report="$tmpdir/filesystem-trim-report.json"
filesystem_trim_receipt="$tmpdir/filesystem-trim-receipt.json"

jq -n '{
  filesystems: {
    scratch: {
      mountpoint: "/scratch",
      fsType: "xfs",
      operation: "trim"
    }
  },
  apply: {}
}' > "$filesystem_trim_spec"

if PATH="$filesystem_trim_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$filesystem_trim_spec" \
  --execute \
  --report-out "$filesystem_trim_report" \
  --receipt-out "$filesystem_trim_receipt" \
  --json > "$filesystem_trim_json"; then
  echo "expected synthetic filesystem trim failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 2
  and .commandSummary.commandCount == 3
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "filesystem:scratch:inspect"
  and .executionResults[0].argv == ["disk-nix", "inspect", "/scratch"]
  and .executionResults[1].success == true
  and .executionResults[1].actionId == "filesystems:scratch:trim"
  and .executionResults[1].argv == ["disk-nix", "inspect", "/scratch"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 83
  and .executionResults[2].argv == ["fstrim", "-v", "/scratch"]
  and (.executionResults[2].stderr | contains("synthetic filesystem trim failure"))
  and .partialExecutionRecovery.completedActionIds == ["filesystem:scratch:inspect"]
  and .partialExecutionRecovery.failedActionId == "filesystems:scratch:trim"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["fstrim", "-v", "/scratch"]
  and .partialExecutionRecovery.retryReviewActionIds == ["filesystems:scratch:trim"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["disk-nix", "filesystems", "--json"]))
    and (.notes | any(contains("filesystem changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "inspect", "scratch", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["disk-nix", "filesystems", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$filesystem_trim_json" >/dev/null

cmp "$filesystem_trim_json" "$filesystem_trim_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.completedActionIds == ["filesystem:scratch:inspect"]
  and .report.partialExecutionRecovery.failedActionId == "filesystems:scratch:trim"
  and .report.partialExecutionRecovery.failedCommand == ["fstrim", "-v", "/scratch"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$filesystem_trim_receipt" >/dev/null

filesystem_check_tools="$tmpdir/fake-filesystem-check-tools"
mkdir -p "$filesystem_check_tools"

cat > "$filesystem_check_tools/e2fsck" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "-n /dev/disk/by-label/home" ]]; then
  echo "synthetic filesystem check failure for disk-nix recovery coverage" >&2
  exit 84
fi
printf '{}\n'
EOF

chmod +x "$filesystem_check_tools/e2fsck"

filesystem_check_spec="$tmpdir/filesystem-check-spec.json"
filesystem_check_json="$tmpdir/filesystem-check-apply.json"
filesystem_check_report="$tmpdir/filesystem-check-report.json"
filesystem_check_receipt="$tmpdir/filesystem-check-receipt.json"

jq -n '{
  filesystems: {
    home: {
      mountpoint: "/home",
      device: "/dev/disk/by-label/home",
      fsType: "ext4",
      operation: "check"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$filesystem_check_spec"

if PATH="$filesystem_check_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$filesystem_check_spec" \
  --execute \
  --report-out "$filesystem_check_report" \
  --receipt-out "$filesystem_check_receipt" \
  --json > "$filesystem_check_json"; then
  echo "expected synthetic filesystem check failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 2
  and .commandSummary.commandCount == 3
  and .commandSummary.mutatingCount == 0
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "filesystem:home:inspect"
  and .executionResults[0].argv == ["disk-nix", "inspect", "/home"]
  and .executionResults[1].success == true
  and .executionResults[1].actionId == "filesystems:home:check"
  and .executionResults[1].argv == ["disk-nix", "inspect", "/home"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 84
  and .executionResults[2].argv == ["e2fsck", "-n", "/dev/disk/by-label/home"]
  and (.executionResults[2].stderr | contains("synthetic filesystem check failure"))
  and .partialExecutionRecovery.completedActionIds == ["filesystem:home:inspect"]
  and .partialExecutionRecovery.failedActionId == "filesystems:home:check"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["e2fsck", "-n", "/dev/disk/by-label/home"]
  and .partialExecutionRecovery.retryReviewActionIds == ["filesystems:home:check"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["blkid", "/dev/disk/by-label/home"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-label/home", "--json"]))
    and (.notes | any(contains("filesystem changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "inspect", "home", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["blkid", "/dev/disk/by-label/home"]))
  ))
' "$filesystem_check_json" >/dev/null

cmp "$filesystem_check_json" "$filesystem_check_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.completedActionIds == ["filesystem:home:inspect"]
  and .report.partialExecutionRecovery.failedActionId == "filesystems:home:check"
  and .report.partialExecutionRecovery.failedCommand == ["e2fsck", "-n", "/dev/disk/by-label/home"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$filesystem_check_receipt" >/dev/null

filesystem_repair_tools="$tmpdir/fake-filesystem-repair-tools"
mkdir -p "$filesystem_repair_tools"

cat > "$filesystem_repair_tools/findmnt" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$filesystem_repair_tools/btrfs" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "check --repair /dev/disk/by-label/data" ]]; then
  echo "synthetic filesystem repair failure for disk-nix recovery coverage" >&2
  exit 85
fi
printf '{}\n'
EOF

chmod +x "$filesystem_repair_tools/findmnt" "$filesystem_repair_tools/btrfs"

filesystem_repair_spec="$tmpdir/filesystem-repair-spec.json"
filesystem_repair_json="$tmpdir/filesystem-repair-apply.json"
filesystem_repair_report="$tmpdir/filesystem-repair-report.json"
filesystem_repair_receipt="$tmpdir/filesystem-repair-receipt.json"

jq -n '{
  filesystems: {
    data: {
      mountpoint: "/data",
      device: "/dev/disk/by-label/data",
      fsType: "btrfs",
      operation: "repair"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$filesystem_repair_spec"

if PATH="$filesystem_repair_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$filesystem_repair_spec" \
  --execute \
  --report-out "$filesystem_repair_report" \
  --receipt-out "$filesystem_repair_receipt" \
  --json > "$filesystem_repair_json"; then
  echo "expected synthetic filesystem repair failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 2
  and .commandSummary.commandCount == 4
  and .commandSummary.mutatingCount == 1
  and (.executionResults | length) == 4
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "filesystem:data:inspect"
  and .executionResults[0].argv == ["disk-nix", "inspect", "/data"]
  and .executionResults[1].success == true
  and .executionResults[1].actionId == "filesystems:data:repair"
  and .executionResults[1].argv == ["disk-nix", "inspect", "/data"]
  and .executionResults[2].success == true
  and .executionResults[2].actionId == "filesystems:data:repair"
  and .executionResults[2].argv == ["findmnt", "--json", "--target", "/data"]
  and .executionResults[3].success == false
  and .executionResults[3].statusCode == 85
  and .executionResults[3].argv == ["btrfs", "check", "--repair", "/dev/disk/by-label/data"]
  and (.executionResults[3].stderr | contains("synthetic filesystem repair failure"))
  and .partialExecutionRecovery.completedActionIds == ["filesystem:data:inspect"]
  and .partialExecutionRecovery.failedActionId == "filesystems:data:repair"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["btrfs", "check", "--repair", "/dev/disk/by-label/data"]
  and .partialExecutionRecovery.retryReviewActionIds == ["filesystems:data:repair"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["findmnt", "--json", "--target", "/data"]))
    and (.commands | any(.argv == ["blkid", "/dev/disk/by-label/data"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-label/data", "--json"]))
    and (.notes | any(contains("filesystem changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["blkid", "/dev/disk/by-label/data"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$filesystem_repair_json" >/dev/null

cmp "$filesystem_repair_json" "$filesystem_repair_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.completedActionIds == ["filesystem:data:inspect"]
  and .report.partialExecutionRecovery.failedActionId == "filesystems:data:repair"
  and .report.partialExecutionRecovery.failedCommand == ["btrfs", "check", "--repair", "/dev/disk/by-label/data"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$filesystem_repair_receipt" >/dev/null

filesystem_property_tools="$tmpdir/fake-filesystem-property-tools"
mkdir -p "$filesystem_property_tools"

cat > "$filesystem_property_tools/xfs_admin" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "-L scratch-new /dev/disk/by-label/scratch-old" ]]; then
  echo "synthetic filesystem property failure for disk-nix recovery coverage" >&2
  exit 86
fi
printf '{}\n'
EOF

cat > "$filesystem_property_tools/blkid" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

chmod +x "$filesystem_property_tools/xfs_admin" "$filesystem_property_tools/blkid"

filesystem_property_spec="$tmpdir/filesystem-property-spec.json"
filesystem_property_json="$tmpdir/filesystem-property-apply.json"
filesystem_property_report="$tmpdir/filesystem-property-report.json"
filesystem_property_receipt="$tmpdir/filesystem-property-receipt.json"

jq -n '{
  filesystems: {
    scratch: {
      mountpoint: "/scratch",
      device: "/dev/disk/by-label/scratch-old",
      fsType: "xfs",
      properties: {
        label: "scratch-new"
      }
    }
  },
  apply: {
    allowOffline: true,
    allowPropertyChanges: true
  }
}' > "$filesystem_property_spec"

if PATH="$filesystem_property_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$filesystem_property_spec" \
  --execute \
  --report-out "$filesystem_property_report" \
  --receipt-out "$filesystem_property_receipt" \
  --json > "$filesystem_property_json"; then
  echo "expected synthetic filesystem property failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 2
  and .commandSummary.commandCount == 3
  and .commandSummary.mutatingCount == 1
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "filesystem:scratch:inspect"
  and .executionResults[0].argv == ["disk-nix", "inspect", "/scratch"]
  and .executionResults[1].success == true
  and .executionResults[1].actionId == "filesystems:scratch:set-property:label"
  and .executionResults[1].argv == ["disk-nix", "inspect", "/scratch"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 86
  and .executionResults[2].argv == ["xfs_admin", "-L", "scratch-new", "/dev/disk/by-label/scratch-old"]
  and (.executionResults[2].stderr | contains("synthetic filesystem property failure"))
  and .partialExecutionRecovery.completedActionIds == ["filesystem:scratch:inspect"]
  and .partialExecutionRecovery.failedActionId == "filesystems:scratch:set-property:label"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["xfs_admin", "-L", "scratch-new", "/dev/disk/by-label/scratch-old"]
  and .partialExecutionRecovery.retryReviewActionIds == ["filesystems:scratch:set-property:label"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["blkid", "/dev/disk/by-label/scratch-old"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-label/scratch-old", "--json"]))
    and (.notes | any(contains("filesystem changes")))
    and (.notes | any(contains("labels")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "inspect", "scratch", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["blkid", "/dev/disk/by-label/scratch-old"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$filesystem_property_json" >/dev/null

cmp "$filesystem_property_json" "$filesystem_property_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.completedActionIds == ["filesystem:scratch:inspect"]
  and .report.partialExecutionRecovery.failedActionId == "filesystems:scratch:set-property:label"
  and .report.partialExecutionRecovery.failedCommand == ["xfs_admin", "-L", "scratch-new", "/dev/disk/by-label/scratch-old"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$filesystem_property_receipt" >/dev/null

swap_label_tools="$tmpdir/fake-swap-label-tools"
mkdir -p "$swap_label_tools"

cat > "$swap_label_tools/swaplabel" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--label swap-new /dev/disk/by-label/swap-old" ]]; then
  echo "synthetic swap label failure for disk-nix recovery coverage" >&2
  exit 75
fi
printf '{}\n'
EOF

cat > "$swap_label_tools/swapon" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$swap_label_tools/blkid" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

chmod +x "$swap_label_tools/swaplabel" "$swap_label_tools/swapon" "$swap_label_tools/blkid"

swap_label_spec="$tmpdir/swap-label-spec.json"
swap_label_json="$tmpdir/swap-label-apply.json"
swap_label_report="$tmpdir/swap-label-report.json"
swap_label_receipt="$tmpdir/swap-label-receipt.json"

jq -n '{
  swaps: {
    primary: {
      device: "/dev/disk/by-label/swap-old",
      properties: {
        label: "swap-new"
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$swap_label_spec"

if PATH="$swap_label_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$swap_label_spec" \
  --execute \
  --report-out "$swap_label_report" \
  --receipt-out "$swap_label_receipt" \
  --json > "$swap_label_json"; then
  echo "expected synthetic swap label failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 3
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "swaps:primary:inspect"
  and .executionResults[0].argv == ["disk-nix", "inspect", "/dev/disk/by-label/swap-old"]
  and .executionResults[1].success == true
  and .executionResults[1].actionId == "swaps:primary:set-property:label"
  and .executionResults[1].argv == ["disk-nix", "inspect", "/dev/disk/by-label/swap-old"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 75
  and .executionResults[2].argv == ["swaplabel", "--label", "swap-new", "/dev/disk/by-label/swap-old"]
  and (.executionResults[2].stderr | contains("synthetic swap label failure"))
  and .partialExecutionRecovery.completedActionIds == ["swaps:primary:inspect"]
  and .partialExecutionRecovery.failedActionId == "swaps:primary:set-property:label"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["swaplabel", "--label", "swap-new", "/dev/disk/by-label/swap-old"]
  and .partialExecutionRecovery.retryReviewActionIds == ["swaps:primary:set-property:label"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["swapon", "--show", "--bytes", "--raw"]))
    and (.commands | any(.argv == ["blkid", "/dev/disk/by-label/swap-old"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-label/swap-old", "--json"]))
    and (.notes | any(contains("swap changes")))
    and (.notes | any(contains("resume")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["swapon", "--show", "--bytes", "--raw"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["blkid", "/dev/disk/by-label/swap-old"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$swap_label_json" >/dev/null

cmp "$swap_label_json" "$swap_label_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.completedActionIds == ["swaps:primary:inspect"]
  and .report.partialExecutionRecovery.failedActionId == "swaps:primary:set-property:label"
  and .report.partialExecutionRecovery.failedCommand == ["swaplabel", "--label", "swap-new", "/dev/disk/by-label/swap-old"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$swap_label_receipt" >/dev/null

zram_rescan_tools="$tmpdir/fake-zram-rescan-tools"
mkdir -p "$zram_rescan_tools"

cat > "$zram_rescan_tools/zramctl" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--bytes --raw --noheadings --output-all" ]]; then
  echo "synthetic zram rescan failure for disk-nix recovery coverage" >&2
  exit 87
fi
printf '{}\n'
EOF

cat > "$zram_rescan_tools/swapon" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

chmod +x "$zram_rescan_tools/zramctl" "$zram_rescan_tools/swapon"

zram_rescan_spec="$tmpdir/zram-rescan-spec.json"
zram_rescan_json="$tmpdir/zram-rescan-apply.json"
zram_rescan_report="$tmpdir/zram-rescan-report.json"
zram_rescan_receipt="$tmpdir/zram-rescan-receipt.json"

jq -n '{
  zram: {
    operation: "rescan"
  },
  apply: {
    allowOffline: true
  }
}' > "$zram_rescan_spec"

if PATH="$zram_rescan_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$zram_rescan_spec" \
  --execute \
  --report-out "$zram_rescan_report" \
  --receipt-out "$zram_rescan_receipt" \
  --json > "$zram_rescan_json"; then
  echo "expected synthetic zram rescan failure to fail apply" >&2
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
  and .executionResults[0].actionId == "zram:rescan"
  and .executionResults[0].argv == ["zramctl", "--bytes", "--raw", "--noheadings", "--output-all"]
  and (.executionResults[0].stderr | contains("synthetic zram rescan failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "zram:rescan"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["zramctl", "--bytes", "--raw", "--noheadings", "--output-all"]
  and .partialExecutionRecovery.retryReviewActionIds == ["zram:rescan"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "review-execution-failure"
    and (.notes | any(contains("zram:rescan")))
    and (.notes | any(contains("synthetic zram rescan failure")))
  ))
  and (.recoveryActions | any(
    .kind == "inspect-current-state"
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "resume-after-fix"))
' "$zram_rescan_json" >/dev/null

cmp "$zram_rescan_json" "$zram_rescan_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "zram:rescan"
  and .report.partialExecutionRecovery.failedCommand == ["zramctl", "--bytes", "--raw", "--noheadings", "--output-all"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$zram_rescan_receipt" >/dev/null

zram_property_tools="$tmpdir/fake-zram-property-tools"
mkdir -p "$zram_property_tools"
zram_property_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$zram_property_tools/zramctl" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--bytes --raw --noheadings --output-all" ]]; then
  echo "synthetic zram property inventory failure for disk-nix recovery coverage" >&2
  exit 88
fi
printf '{}\n'
EOF

cat > "$zram_property_tools/swapon" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$zram_property_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$zram_property_disk_nix" "\$@"
EOF

chmod +x "$zram_property_tools/zramctl" "$zram_property_tools/swapon" "$zram_property_tools/disk-nix"

zram_property_spec="$tmpdir/zram-property-spec.json"
zram_property_json="$tmpdir/zram-property-apply.json"
zram_property_report="$tmpdir/zram-property-report.json"
zram_property_receipt="$tmpdir/zram-property-receipt.json"

jq -n '{
  zram: {
    enable: true,
    properties: {
      algorithm: "zstd"
    }
  },
  apply: {
    allowOffline: true,
    allowPropertyChanges: true
  }
}' > "$zram_property_spec"

if PATH="$zram_property_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$zram_property_spec" \
  --execute \
  --report-out "$zram_property_report" \
  --receipt-out "$zram_property_receipt" \
  --json > "$zram_property_json"; then
  echo "expected synthetic zram property failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 6
  and .commandSummary.mutatingCount == 0
  and (.executionResults | length) == 1
  and .executionResults[0].success == false
  and .executionResults[0].statusCode == 88
  and .executionResults[0].actionId == "zram:inspect"
  and .executionResults[0].argv == ["zramctl", "--bytes", "--raw", "--noheadings", "--output-all"]
  and (.executionResults[0].stderr | contains("synthetic zram property inventory failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "zram:inspect"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["zramctl", "--bytes", "--raw", "--noheadings", "--output-all"]
  and .partialExecutionRecovery.retryReviewActionIds == ["zram:inspect", "zram:set-property:algorithm"]
  and .partialExecutionRecovery.remainingActionIds == ["zram:set-property:algorithm"]
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "review-execution-failure"
    and (.notes | any(contains("zram:inspect")))
    and (.notes | any(contains("synthetic zram property inventory failure")))
  ))
  and (.recoveryActions | any(
    .kind == "inspect-current-state"
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "resume-after-fix"))
' "$zram_property_json" >/dev/null

cmp "$zram_property_json" "$zram_property_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "zram:inspect"
  and .report.partialExecutionRecovery.failedCommand == ["zramctl", "--bytes", "--raw", "--noheadings", "--output-all"]
  and .report.partialExecutionRecovery.retryReviewActionIds == ["zram:inspect", "zram:set-property:algorithm"]
  and .report.partialExecutionRecovery.remainingActionIds == ["zram:set-property:algorithm"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$zram_property_receipt" >/dev/null

loop_rescan_tools="$tmpdir/fake-loop-rescan-tools"
mkdir -p "$loop_rescan_tools"
loop_rescan_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$loop_rescan_tools/losetup" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--json --list /dev/loop7" ]]; then
  echo "synthetic loop rescan failure for disk-nix recovery coverage" >&2
  exit 86
fi
printf '{}\n'
EOF

cat > "$loop_rescan_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$loop_rescan_disk_nix" "\$@"
EOF

chmod +x "$loop_rescan_tools/losetup" "$loop_rescan_tools/disk-nix"

loop_rescan_spec="$tmpdir/loop-rescan-spec.json"
loop_rescan_json="$tmpdir/loop-rescan-apply.json"
loop_rescan_report="$tmpdir/loop-rescan-report.json"
loop_rescan_receipt="$tmpdir/loop-rescan-receipt.json"

jq -n '{
  loopDevices: {
    "/dev/loop7": {
      operation: "rescan"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$loop_rescan_spec"

if PATH="$loop_rescan_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$loop_rescan_spec" \
  --execute \
  --report-out "$loop_rescan_report" \
  --receipt-out "$loop_rescan_receipt" \
  --json > "$loop_rescan_json"; then
  echo "expected synthetic loop rescan failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and .commandSummary.mutatingCount == 0
  and (.executionResults | length) == 1
  and .executionResults[0].success == false
  and .executionResults[0].statusCode == 86
  and .executionResults[0].actionId == "loopdevices:/dev/loop7:rescan"
  and .executionResults[0].argv == ["losetup", "--json", "--list", "/dev/loop7"]
  and (.executionResults[0].stderr | contains("synthetic loop rescan failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "loopdevices:/dev/loop7:rescan"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["losetup", "--json", "--list", "/dev/loop7"]
  and .partialExecutionRecovery.retryReviewActionIds == ["loopdevices:/dev/loop7:rescan"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["losetup", "--json", "--list", "/dev/loop7"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/loop7", "--json"]))
    and (.notes | any(contains("local mapping changes")))
    and (.notes | any(contains("modeled consumers")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["losetup", "--json", "--list", "/dev/loop7"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/loop7", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["losetup", "--json", "--list", "/dev/loop7"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/loop7", "--json"]))
  ))
' "$loop_rescan_json" >/dev/null

cmp "$loop_rescan_json" "$loop_rescan_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "loopdevices:/dev/loop7:rescan"
  and .report.partialExecutionRecovery.failedCommand == ["losetup", "--json", "--list", "/dev/loop7"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$loop_rescan_receipt" >/dev/null
