fake_tools="$tmpdir/fake-tools"
mkdir -p "$fake_tools"

cat > "$fake_tools/lvextend" <<'EOF'
#!/usr/bin/env bash
printf 'synthetic lvextend success: %s\n' "$*"
exit 0
EOF

cat > "$fake_tools/lvs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$fake_tools/resize2fs" <<'EOF'
#!/usr/bin/env bash
echo "synthetic resize failure for disk-nix partial recovery smoke" >&2
exit 73
EOF

chmod +x "$fake_tools/lvextend" "$fake_tools/lvs" "$fake_tools/resize2fs"

spec="$tmpdir/spec.json"
apply_json="$tmpdir/apply.json"
report="$tmpdir/apply-report.json"
receipt="$tmpdir/apply-receipt.json"

jq -n '{
  version: 1,
  volumes: {
    "vg0/root": {
      operation: "grow",
      target: "vg0/root",
      desiredSize: "50GiB"
    }
  },
  filesystems: {
    root: {
      operation: "grow",
      device: "vg0/root",
      fsType: "ext4",
      desiredSize: "50GiB",
      resizePolicy: "grow-only"
    }
  },
  apply: {
    allowGrow: true
  }
}' > "$spec"

if PATH="$fake_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --receipt-out "$receipt" \
  --json > "$apply_json"; then
  echo "expected synthetic resize failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and (.executionResults | length) == 4
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["lvs", "--reportformat", "json", "vg0/root"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["lvextend", "--resizefs", "--size", "50GiB", "vg0/root"]
  and .executionResults[2].success == true
  and .executionResults[2].argv == ["disk-nix", "inspect", "root"]
  and .executionResults[3].success == false
  and .executionResults[3].statusCode == 73
  and .executionResults[3].argv == ["resize2fs", "vg0/root", "50GiB"]
  and (.executionResults[3].stderr | contains("synthetic resize failure"))
  and .partialExecutionRecovery.completedActionIds == ["volumes:vg0/root:grow"]
  and .partialExecutionRecovery.failedActionId == "filesystem:root:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["resize2fs", "vg0/root", "50GiB"]
  and .partialExecutionRecovery.retryReviewActionIds == ["filesystem:root:grow"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(.kind == "domain-recovery"))
  and (.recoveryActions | any(.kind == "roll-forward-review"))
  and (.recoveryActions | any(.kind == "rollback-review"))
' "$apply_json" >/dev/null

cmp "$apply_json" "$report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.completedActionIds == ["volumes:vg0/root:grow"]
  and .report.partialExecutionRecovery.failedActionId == "filesystem:root:grow"
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$receipt" >/dev/null

lvm_grow_tools="$tmpdir/fake-lvm-grow-tools"
mkdir -p "$lvm_grow_tools"

cat > "$lvm_grow_tools/lvs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$lvm_grow_tools/vgs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$lvm_grow_tools/pvs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$lvm_grow_tools/lvextend" <<'EOF'
#!/usr/bin/env bash
echo "synthetic LVM grow failure for disk-nix recovery coverage" >&2
exit 79
EOF

chmod +x "$lvm_grow_tools/lvs" "$lvm_grow_tools/vgs" "$lvm_grow_tools/pvs" "$lvm_grow_tools/lvextend"

lvm_grow_spec="$tmpdir/lvm-grow-spec.json"
lvm_grow_json="$tmpdir/lvm-grow-apply.json"
lvm_grow_report="$tmpdir/lvm-grow-report.json"
lvm_grow_receipt="$tmpdir/lvm-grow-receipt.json"

jq -n '{
  volumes: {
    root: {
      target: "vg0/root",
      operation: "grow",
      desiredSize: "50GiB"
    }
  },
  apply: {
    allowGrow: true
  }
}' > "$lvm_grow_spec"

if PATH="$lvm_grow_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$lvm_grow_spec" \
  --execute \
  --report-out "$lvm_grow_report" \
  --receipt-out "$lvm_grow_receipt" \
  --json > "$lvm_grow_json"; then
  echo "expected synthetic LVM grow failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["lvs", "--reportformat", "json", "vg0/root"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 79
  and .executionResults[1].argv == ["lvextend", "--resizefs", "--size", "50GiB", "vg0/root"]
  and (.executionResults[1].stderr | contains("synthetic LVM grow failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "volumes:root:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["lvextend", "--resizefs", "--size", "50GiB", "vg0/root"]
  and .partialExecutionRecovery.retryReviewActionIds == ["volumes:root:grow"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "vg0/root"]))
    and (.commands | any(.argv == ["vgs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "vg0/root", "--json"]))
    and (.notes | any(contains("LVM changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "vg0/root"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$lvm_grow_json" >/dev/null

cmp "$lvm_grow_json" "$lvm_grow_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "volumes:root:grow"
  and .report.partialExecutionRecovery.failedCommand == ["lvextend", "--resizefs", "--size", "50GiB", "vg0/root"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$lvm_grow_receipt" >/dev/null

lvm_thin_create_tools="$tmpdir/fake-lvm-thin-create-tools"
mkdir -p "$lvm_thin_create_tools"

cat > "$lvm_thin_create_tools/vgs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$lvm_thin_create_tools/lvcreate" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--type thin-pool --size 100GiB --name newpool vg0" ]]; then
  echo "synthetic LVM thin-pool create failure for disk-nix recovery coverage" >&2
  exit 86
fi
printf '{}\n'
exit 0
EOF

chmod +x "$lvm_thin_create_tools/vgs" "$lvm_thin_create_tools/lvcreate"

lvm_thin_create_spec="$tmpdir/lvm-thin-create-spec.json"
lvm_thin_create_json="$tmpdir/lvm-thin-create-apply.json"
lvm_thin_create_report="$tmpdir/lvm-thin-create-report.json"
lvm_thin_create_receipt="$tmpdir/lvm-thin-create-receipt.json"

jq -n '{
  thinPools: {
    "vg0/newpool": {
      operation: "create",
      desiredSize: "100GiB"
    }
  }
}' > "$lvm_thin_create_spec"

if PATH="$lvm_thin_create_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$lvm_thin_create_spec" \
  --execute \
  --report-out "$lvm_thin_create_report" \
  --receipt-out "$lvm_thin_create_receipt" \
  --json > "$lvm_thin_create_json"; then
  echo "expected synthetic LVM thin-pool create failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and .commandSummary.mutatingCount == 1
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["vgs", "--reportformat", "json"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 86
  and .executionResults[1].argv == ["lvcreate", "--type", "thin-pool", "--size", "100GiB", "--name", "newpool", "vg0"]
  and (.executionResults[1].stderr | contains("synthetic LVM thin-pool create failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "thinpools:vg0/newpool:create"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["lvcreate", "--type", "thin-pool", "--size", "100GiB", "--name", "newpool", "vg0"]
  and .partialExecutionRecovery.retryReviewActionIds == ["thinpools:vg0/newpool:create"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "vg0/newpool"]))
    and (.commands | any(.argv == ["vgs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "vg0/newpool", "--json"]))
    and (.notes | any(contains("LVM changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "vg0/newpool"]))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-o", "lv_name,lv_size,data_percent,metadata_percent,seg_monitor", "vg0/newpool"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$lvm_thin_create_json" >/dev/null

cmp "$lvm_thin_create_json" "$lvm_thin_create_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "thinpools:vg0/newpool:create"
  and .report.partialExecutionRecovery.failedCommand == ["lvcreate", "--type", "thin-pool", "--size", "100GiB", "--name", "newpool", "vg0"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$lvm_thin_create_receipt" >/dev/null

lvm_thin_grow_tools="$tmpdir/fake-lvm-thin-grow-tools"
mkdir -p "$lvm_thin_grow_tools"

cat > "$lvm_thin_grow_tools/lvs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$lvm_thin_grow_tools/vgs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$lvm_thin_grow_tools/pvs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$lvm_thin_grow_tools/lvextend" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--size 500GiB vg0/thinpool" ]]; then
  echo "synthetic LVM thin-pool grow failure for disk-nix recovery coverage" >&2
  exit 85
fi
printf '{}\n'
exit 0
EOF

chmod +x "$lvm_thin_grow_tools/lvs" "$lvm_thin_grow_tools/vgs" "$lvm_thin_grow_tools/pvs" "$lvm_thin_grow_tools/lvextend"

lvm_thin_grow_spec="$tmpdir/lvm-thin-grow-spec.json"
lvm_thin_grow_json="$tmpdir/lvm-thin-grow-apply.json"
lvm_thin_grow_report="$tmpdir/lvm-thin-grow-report.json"
lvm_thin_grow_receipt="$tmpdir/lvm-thin-grow-receipt.json"

jq -n '{
  thinPools: {
    "vg0/thinpool": {
      operation: "grow",
      desiredSize: "500GiB"
    }
  },
  apply: {
    allowGrow: true
  }
}' > "$lvm_thin_grow_spec"

if PATH="$lvm_thin_grow_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$lvm_thin_grow_spec" \
  --execute \
  --report-out "$lvm_thin_grow_report" \
  --receipt-out "$lvm_thin_grow_receipt" \
  --json > "$lvm_thin_grow_json"; then
  echo "expected synthetic LVM thin-pool grow failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and .commandSummary.mutatingCount == 1
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["lvs", "--reportformat", "json", "-o", "lv_name,lv_size,data_percent,metadata_percent,seg_monitor", "vg0/thinpool"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 85
  and .executionResults[1].argv == ["lvextend", "--size", "500GiB", "vg0/thinpool"]
  and (.executionResults[1].stderr | contains("synthetic LVM thin-pool grow failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "thinpools:vg0/thinpool:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["lvextend", "--size", "500GiB", "vg0/thinpool"]
  and .partialExecutionRecovery.retryReviewActionIds == ["thinpools:vg0/thinpool:grow"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "vg0/thinpool"]))
    and (.commands | any(.argv == ["vgs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "vg0/thinpool", "--json"]))
    and (.notes | any(contains("LVM changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "vg0/thinpool"]))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-o", "lv_name,lv_size,data_percent,metadata_percent,seg_monitor", "vg0/thinpool"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$lvm_thin_grow_json" >/dev/null

cmp "$lvm_thin_grow_json" "$lvm_thin_grow_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "thinpools:vg0/thinpool:grow"
  and .report.partialExecutionRecovery.failedCommand == ["lvextend", "--size", "500GiB", "vg0/thinpool"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$lvm_thin_grow_receipt" >/dev/null

xfs_grow_tools="$tmpdir/fake-xfs-grow-tools"
mkdir -p "$xfs_grow_tools"

cat > "$xfs_grow_tools/findmnt" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$xfs_grow_tools/xfs_growfs" <<'EOF'
#!/usr/bin/env bash
echo "synthetic XFS grow failure for disk-nix recovery coverage" >&2
exit 80
EOF

chmod +x "$xfs_grow_tools/findmnt" "$xfs_grow_tools/xfs_growfs"

xfs_grow_spec="$tmpdir/xfs-grow-spec.json"
xfs_grow_json="$tmpdir/xfs-grow-apply.json"
xfs_grow_report="$tmpdir/xfs-grow-report.json"
xfs_grow_receipt="$tmpdir/xfs-grow-receipt.json"

jq -n '{
  filesystems: {
    root: {
      mountpoint: "/",
      fsType: "xfs",
      resizePolicy: "grow-only"
    }
  },
  apply: {
    allowGrow: true
  }
}' > "$xfs_grow_spec"

if PATH="$xfs_grow_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$xfs_grow_spec" \
  --execute \
  --report-out "$xfs_grow_report" \
  --receipt-out "$xfs_grow_receipt" \
  --json > "$xfs_grow_json"; then
  echo "expected synthetic XFS grow failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["disk-nix", "inspect", "/"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 80
  and .executionResults[1].argv == ["xfs_growfs", "/"]
  and (.executionResults[1].stderr | contains("synthetic XFS grow failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "filesystem:root:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["xfs_growfs", "/"]
  and .partialExecutionRecovery.retryReviewActionIds == ["filesystem:root:grow"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["findmnt", "--json", "--target", "/"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/", "--json"]))
    and (.notes | any(contains("filesystem changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["findmnt", "--json", "--target", "/"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$xfs_grow_json" >/dev/null

cmp "$xfs_grow_json" "$xfs_grow_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "filesystem:root:grow"
  and .report.partialExecutionRecovery.failedCommand == ["xfs_growfs", "/"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$xfs_grow_receipt" >/dev/null

btrfs_scrub_tools="$tmpdir/fake-btrfs-scrub-tools"
mkdir -p "$btrfs_scrub_tools"

cat > "$btrfs_scrub_tools/findmnt" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$btrfs_scrub_tools/btrfs" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "scrub start -B /data" ]]; then
  echo "synthetic Btrfs scrub failure for disk-nix recovery coverage" >&2
  exit 81
fi
printf '{}\n'
EOF

chmod +x "$btrfs_scrub_tools/findmnt" "$btrfs_scrub_tools/btrfs"

btrfs_scrub_spec="$tmpdir/btrfs-scrub-spec.json"
btrfs_scrub_json="$tmpdir/btrfs-scrub-apply.json"
btrfs_scrub_report="$tmpdir/btrfs-scrub-report.json"
btrfs_scrub_receipt="$tmpdir/btrfs-scrub-receipt.json"

jq -n '{
  filesystems: {
    data: {
      mountpoint: "/data",
      fsType: "btrfs",
      operation: "scrub"
    }
  },
  apply: {}
}' > "$btrfs_scrub_spec"

if PATH="$btrfs_scrub_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$btrfs_scrub_spec" \
  --execute \
  --report-out "$btrfs_scrub_report" \
  --receipt-out "$btrfs_scrub_receipt" \
  --json > "$btrfs_scrub_json"; then
  echo "expected synthetic Btrfs scrub failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 2
  and .commandSummary.commandCount == 3
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "filesystem:data:inspect"
  and .executionResults[0].argv == ["disk-nix", "inspect", "/data"]
  and .executionResults[1].success == true
  and .executionResults[1].actionId == "filesystems:data:scrub"
  and .executionResults[1].argv == ["disk-nix", "inspect", "/data"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 81
  and .executionResults[2].argv == ["btrfs", "scrub", "start", "-B", "/data"]
  and (.executionResults[2].stderr | contains("synthetic Btrfs scrub failure"))
  and .partialExecutionRecovery.completedActionIds == ["filesystem:data:inspect"]
  and .partialExecutionRecovery.failedActionId == "filesystems:data:scrub"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["btrfs", "scrub", "start", "-B", "/data"]
  and .partialExecutionRecovery.retryReviewActionIds == ["filesystems:data:scrub"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["findmnt", "--json", "--target", "/data"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/data", "--json"]))
    and (.notes | any(contains("filesystem changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["findmnt", "--json", "--target", "/data"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/data", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$btrfs_scrub_json" >/dev/null

cmp "$btrfs_scrub_json" "$btrfs_scrub_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.completedActionIds == ["filesystem:data:inspect"]
  and .report.partialExecutionRecovery.failedActionId == "filesystems:data:scrub"
  and .report.partialExecutionRecovery.failedCommand == ["btrfs", "scrub", "start", "-B", "/data"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$btrfs_scrub_receipt" >/dev/null

btrfs_rebalance_tools="$tmpdir/fake-btrfs-rebalance-tools"
mkdir -p "$btrfs_rebalance_tools"

cat > "$btrfs_rebalance_tools/findmnt" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$btrfs_rebalance_tools/btrfs" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "balance start -dusage=50 -musage=75 /data" ]]; then
  echo "synthetic Btrfs rebalance failure for disk-nix recovery coverage" >&2
  exit 82
fi
printf '{}\n'
EOF

chmod +x "$btrfs_rebalance_tools/findmnt" "$btrfs_rebalance_tools/btrfs"

btrfs_rebalance_spec="$tmpdir/btrfs-rebalance-spec.json"
btrfs_rebalance_json="$tmpdir/btrfs-rebalance-apply.json"
btrfs_rebalance_report="$tmpdir/btrfs-rebalance-report.json"
btrfs_rebalance_receipt="$tmpdir/btrfs-rebalance-receipt.json"

jq -n '{
  filesystems: {
    data: {
      mountpoint: "/data",
      fsType: "btrfs",
      operation: "rebalance",
      properties: {
        "balance.data": "usage=50",
        "balance.metadata": "usage=75"
      }
    }
  },
  apply: {
    allowRebalance: true
  }
}' > "$btrfs_rebalance_spec"

if PATH="$btrfs_rebalance_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$btrfs_rebalance_spec" \
  --execute \
  --report-out "$btrfs_rebalance_report" \
  --receipt-out "$btrfs_rebalance_receipt" \
  --json > "$btrfs_rebalance_json"; then
  echo "expected synthetic Btrfs rebalance failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 2
  and .commandSummary.commandCount == 3
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "filesystem:data:inspect"
  and .executionResults[0].argv == ["disk-nix", "inspect", "/data"]
  and .executionResults[1].success == true
  and .executionResults[1].actionId == "filesystems:data:rebalance"
  and .executionResults[1].argv == ["disk-nix", "inspect", "/data"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 82
  and .executionResults[2].argv == ["btrfs", "balance", "start", "-dusage=50", "-musage=75", "/data"]
  and (.executionResults[2].stderr | contains("synthetic Btrfs rebalance failure"))
  and .partialExecutionRecovery.completedActionIds == ["filesystem:data:inspect"]
  and .partialExecutionRecovery.failedActionId == "filesystems:data:rebalance"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["btrfs", "balance", "start", "-dusage=50", "-musage=75", "/data"]
  and .partialExecutionRecovery.retryReviewActionIds == ["filesystems:data:rebalance"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["findmnt", "--json", "--target", "/data"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/data", "--json"]))
    and (.notes | any(contains("filesystem changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["findmnt", "--json", "--target", "/data"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/data", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$btrfs_rebalance_json" >/dev/null

cmp "$btrfs_rebalance_json" "$btrfs_rebalance_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.completedActionIds == ["filesystem:data:inspect"]
  and .report.partialExecutionRecovery.failedActionId == "filesystems:data:rebalance"
  and .report.partialExecutionRecovery.failedCommand == ["btrfs", "balance", "start", "-dusage=50", "-musage=75", "/data"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$btrfs_rebalance_receipt" >/dev/null

btrfs_replace_tools="$tmpdir/fake-btrfs-replace-tools"
mkdir -p "$btrfs_replace_tools"
btrfs_replace_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$btrfs_replace_tools/findmnt" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$btrfs_replace_tools/btrfs" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "replace start /dev/disk/by-id/old-btrfs-device /dev/disk/by-id/new-btrfs-device /data" ]]; then
  echo "synthetic Btrfs device replacement failure for disk-nix recovery coverage" >&2
  exit 84
fi
printf '{}\n'
EOF

cat > "$btrfs_replace_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$btrfs_replace_disk_nix" "\$@"
EOF

chmod +x "$btrfs_replace_tools/findmnt" "$btrfs_replace_tools/btrfs" "$btrfs_replace_tools/disk-nix"

btrfs_replace_spec="$tmpdir/btrfs-replace-spec.json"
btrfs_replace_json="$tmpdir/btrfs-replace-apply.json"
btrfs_replace_report="$tmpdir/btrfs-replace-report.json"
btrfs_replace_receipt="$tmpdir/btrfs-replace-receipt.json"

jq -n '{
  filesystems: {
    data: {
      mountpoint: "/data",
      fsType: "btrfs",
      replaceDevices: {
        "/dev/disk/by-id/old-btrfs-device": "/dev/disk/by-id/new-btrfs-device"
      }
    }
  },
  apply: {
    allowDeviceReplacement: true
  }
}' > "$btrfs_replace_spec"

if PATH="$btrfs_replace_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$btrfs_replace_spec" \
  --execute \
  --report-out "$btrfs_replace_report" \
  --receipt-out "$btrfs_replace_receipt" \
  --json > "$btrfs_replace_json"; then
  echo "expected synthetic Btrfs device replacement failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 2
  and .commandSummary.commandCount == 3
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "filesystem:data:inspect"
  and .executionResults[0].argv == ["disk-nix", "inspect", "/data"]
  and .executionResults[1].success == true
  and .executionResults[1].actionId == "filesystems:data:replace-device:/dev/disk/by-id/old-btrfs-device"
  and .executionResults[1].argv == ["disk-nix", "inspect", "/data"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 84
  and .executionResults[2].argv == ["btrfs", "replace", "start", "/dev/disk/by-id/old-btrfs-device", "/dev/disk/by-id/new-btrfs-device", "/data"]
  and (.executionResults[2].stderr | contains("synthetic Btrfs device replacement failure"))
  and .partialExecutionRecovery.completedActionIds == ["filesystem:data:inspect"]
  and .partialExecutionRecovery.failedActionId == "filesystems:data:replace-device:/dev/disk/by-id/old-btrfs-device"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["btrfs", "replace", "start", "/dev/disk/by-id/old-btrfs-device", "/dev/disk/by-id/new-btrfs-device", "/data"]
  and .partialExecutionRecovery.retryReviewActionIds == ["filesystems:data:replace-device:/dev/disk/by-id/old-btrfs-device"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["disk-nix", "inspect", "data", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
    and (.notes | any(contains("failed Command command")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "inspect", "data", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/data", "--json"]))
    and (.commands | any(.argv == ["btrfs", "filesystem", "usage", "-b", "/data"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$btrfs_replace_json" >/dev/null

cmp "$btrfs_replace_json" "$btrfs_replace_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.completedActionIds == ["filesystem:data:inspect"]
  and .report.partialExecutionRecovery.failedActionId == "filesystems:data:replace-device:/dev/disk/by-id/old-btrfs-device"
  and .report.partialExecutionRecovery.failedCommand == ["btrfs", "replace", "start", "/dev/disk/by-id/old-btrfs-device", "/dev/disk/by-id/new-btrfs-device", "/data"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$btrfs_replace_receipt" >/dev/null

bcachefs_replace_tools="$tmpdir/fake-bcachefs-replace-tools"
mkdir -p "$bcachefs_replace_tools"
bcachefs_replace_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$bcachefs_replace_tools/bcachefs" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "data rereplicate /bulk" ]]; then
  echo "synthetic bcachefs replacement rereplicate failure for disk-nix recovery coverage" >&2
  exit 85
fi
printf '{}\n'
EOF

cat > "$bcachefs_replace_tools/btrfs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$bcachefs_replace_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$bcachefs_replace_disk_nix" "\$@"
EOF

chmod +x "$bcachefs_replace_tools/bcachefs" "$bcachefs_replace_tools/btrfs" "$bcachefs_replace_tools/disk-nix"

bcachefs_replace_spec="$tmpdir/bcachefs-replace-spec.json"
bcachefs_replace_json="$tmpdir/bcachefs-replace-apply.json"
bcachefs_replace_report="$tmpdir/bcachefs-replace-report.json"
bcachefs_replace_receipt="$tmpdir/bcachefs-replace-receipt.json"

jq -n '{
  filesystems: {
    bulk: {
      mountpoint: "/bulk",
      fsType: "bcachefs",
      replaceDevices: {
        "/dev/disk/by-id/old-bcachefs-device": "/dev/disk/by-id/new-bcachefs-device"
      }
    }
  },
  apply: {
    allowOffline: true,
    allowDeviceReplacement: true
  }
}' > "$bcachefs_replace_spec"

if PATH="$bcachefs_replace_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$bcachefs_replace_spec" \
  --execute \
  --report-out "$bcachefs_replace_report" \
  --receipt-out "$bcachefs_replace_receipt" \
  --json > "$bcachefs_replace_json"; then
  echo "expected synthetic bcachefs replacement rereplicate failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 2
  and .commandSummary.commandCount == 5
  and .commandSummary.mutatingCount == 3
  and .commandSummary.manualReviewCount == 1
  and .commandSummary.readyCount == 5
  and (.commandSummary.missingToolCount // 0) == 0
  and (.commandSummary.readinessIssueCount // 0) == 0
  and (.executionResults | length) == 4
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "filesystem:bulk:inspect"
  and .executionResults[0].argv == ["disk-nix", "inspect", "/bulk"]
  and .executionResults[1].success == true
  and .executionResults[1].actionId == "filesystems:bulk:replace-device:/dev/disk/by-id/old-bcachefs-device"
  and .executionResults[1].argv == ["bcachefs", "fs", "usage", "/bulk"]
  and .executionResults[2].success == true
  and .executionResults[2].actionId == "filesystems:bulk:replace-device:/dev/disk/by-id/old-bcachefs-device"
  and .executionResults[2].argv == ["bcachefs", "device", "add", "/bulk", "/dev/disk/by-id/new-bcachefs-device"]
  and .executionResults[3].success == false
  and .executionResults[3].statusCode == 85
  and .executionResults[3].actionId == "filesystems:bulk:replace-device:/dev/disk/by-id/old-bcachefs-device"
  and .executionResults[3].argv == ["bcachefs", "data", "rereplicate", "/bulk"]
  and (.executionResults[3].stderr | contains("synthetic bcachefs replacement rereplicate failure"))
  and .partialExecutionRecovery.completedActionIds == ["filesystem:bulk:inspect"]
  and .partialExecutionRecovery.failedActionId == "filesystems:bulk:replace-device:/dev/disk/by-id/old-bcachefs-device"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["bcachefs", "data", "rereplicate", "/bulk"]
  and .partialExecutionRecovery.retryReviewActionIds == ["filesystems:bulk:replace-device:/dev/disk/by-id/old-bcachefs-device"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["disk-nix", "inspect", "bulk", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
    and (.notes | any(contains("1 mutating command(s) completed")))
    and (.notes | any(contains("bcachefs data rereplicate /bulk")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["disk-nix", "inspect", "bulk", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/bulk", "--json"]))
    and (.commands | any(.argv == ["btrfs", "filesystem", "usage", "-b", "/bulk"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$bcachefs_replace_json" >/dev/null

cmp "$bcachefs_replace_json" "$bcachefs_replace_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.completedActionIds == ["filesystem:bulk:inspect"]
  and .report.partialExecutionRecovery.failedActionId == "filesystems:bulk:replace-device:/dev/disk/by-id/old-bcachefs-device"
  and .report.partialExecutionRecovery.failedCommand == ["bcachefs", "data", "rereplicate", "/bulk"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$bcachefs_replace_receipt" >/dev/null
