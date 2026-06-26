#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run failure-recovery integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this harness drives
disk-nix in execute mode. This test uses fake storage tools in a temporary
directory and does not mutate real block devices.
MSG
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" jq mktemp chmod mkdir rm cmp; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT

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

rollback_tools="$tmpdir/fake-rollback-tools"
mkdir -p "$rollback_tools"

cat > "$rollback_tools/zfs" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "rollback" ]]; then
  echo "synthetic zfs rollback failure for disk-nix recovery coverage" >&2
  exit 74
fi
printf '{}\n'
EOF

chmod +x "$rollback_tools/zfs"

rollback_spec="$tmpdir/rollback-spec.json"
rollback_json="$tmpdir/rollback-apply.json"
rollback_report="$tmpdir/rollback-report.json"
rollback_receipt="$tmpdir/rollback-receipt.json"

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
}' > "$rollback_spec"

if PATH="$rollback_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$rollback_spec" \
  --execute \
  --report-out "$rollback_report" \
  --receipt-out "$rollback_receipt" \
  --json > "$rollback_json"; then
  echo "expected synthetic ZFS rollback failure to fail apply" >&2
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
  and .executionResults[1].statusCode == 74
  and .executionResults[1].argv == ["zfs", "rollback", "tank/home@before"]
  and (.executionResults[1].stderr | contains("synthetic zfs rollback failure"))
  and .partialExecutionRecovery.failedActionId == "snapshot:tank/home@before:rollback"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["zfs", "rollback", "tank/home@before"]
  and .partialExecutionRecovery.retryReviewActionIds == ["snapshot:tank/home@before:rollback"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
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
' "$rollback_json" >/dev/null

cmp "$rollback_json" "$rollback_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "snapshot:tank/home@before:rollback"
  and .report.partialExecutionRecovery.failedCommand == ["zfs", "rollback", "tank/home@before"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$rollback_receipt" >/dev/null

nvme_create_tools="$tmpdir/fake-nvme-create-tools"
mkdir -p "$nvme_create_tools"

cat > "$nvme_create_tools/nvme" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "create-ns" ]]; then
  echo "synthetic nvme namespace create failure for disk-nix recovery coverage" >&2
  exit 81
fi
printf '{}\n'
EOF

chmod +x "$nvme_create_tools/nvme"

nvme_create_spec="$tmpdir/nvme-create-spec.json"
nvme_create_json="$tmpdir/nvme-create-apply.json"
nvme_create_report="$tmpdir/nvme-create-report.json"
nvme_create_receipt="$tmpdir/nvme-create-receipt.json"

jq -n '{
  nvmeNamespaces: {
    "/dev/nvme0": {
      operation: "create",
      desiredSize: "100G",
      namespaceId: "4",
      controllers: "0x1"
    }
  },
  apply: {
    allowDestructive: true,
    allowOffline: true
  }
}' > "$nvme_create_spec"

if PATH="$nvme_create_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$nvme_create_spec" \
  --execute \
  --report-out "$nvme_create_report" \
  --receipt-out "$nvme_create_receipt" \
  --json > "$nvme_create_json"; then
  echo "expected synthetic NVMe namespace create failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 4
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["nvme", "list-ns", "/dev/nvme0", "--all", "--output-format=json"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 81
  and .executionResults[1].argv == ["nvme", "create-ns", "/dev/nvme0", "--nsze-si", "100G", "--ncap-si", "100G"]
  and (.executionResults[1].stderr | contains("synthetic nvme namespace create failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "nvmenamespaces:/dev/nvme0:create"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["nvme", "create-ns", "/dev/nvme0", "--nsze-si", "100G", "--ncap-si", "100G"]
  and .partialExecutionRecovery.retryReviewActionIds == ["nvmenamespaces:/dev/nvme0:create"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["nvme", "list-ns", "/dev/nvme0", "--all", "--output-format=json"]))
    and (.commands | any(.argv == ["nvme", "list-subsys", "--output-format=json"]))
    and (.notes | any(contains("NVMe namespace changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["nvme", "list-ns", "/dev/nvme0", "--all", "--output-format=json"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["nvme", "list-subsys", "--output-format=json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$nvme_create_json" >/dev/null

cmp "$nvme_create_json" "$nvme_create_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "nvmenamespaces:/dev/nvme0:create"
  and .report.partialExecutionRecovery.failedCommand == ["nvme", "create-ns", "/dev/nvme0", "--nsze-si", "100G", "--ncap-si", "100G"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$nvme_create_receipt" >/dev/null

nvme_grow_tools="$tmpdir/fake-nvme-grow-tools"
mkdir -p "$nvme_grow_tools"

cat > "$nvme_grow_tools/nvme" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "ns-rescan" ]]; then
  echo "synthetic nvme namespace grow rescan failure for disk-nix recovery coverage" >&2
  exit 84
fi
printf '{}\n'
EOF

chmod +x "$nvme_grow_tools/nvme"

nvme_grow_spec="$tmpdir/nvme-grow-spec.json"
nvme_grow_json="$tmpdir/nvme-grow-apply.json"
nvme_grow_report="$tmpdir/nvme-grow-report.json"
nvme_grow_receipt="$tmpdir/nvme-grow-receipt.json"

jq -n '{
  nvmeNamespaces: {
    "/dev/nvme1": {
      operation: "grow"
    }
  },
  apply: {
    allowGrow: true,
    allowOffline: true
  }
}' > "$nvme_grow_spec"

if PATH="$nvme_grow_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$nvme_grow_spec" \
  --execute \
  --report-out "$nvme_grow_report" \
  --receipt-out "$nvme_grow_receipt" \
  --json > "$nvme_grow_json"; then
  echo "expected synthetic NVMe namespace grow failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 5
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["nvme", "list-ns", "/dev/nvme1", "--all", "--output-format=json"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["nvme", "list-subsys", "--output-format=json"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 84
  and .executionResults[2].argv == ["nvme", "ns-rescan", "/dev/nvme1"]
  and (.executionResults[2].stderr | contains("synthetic nvme namespace grow rescan failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "nvmenamespaces:/dev/nvme1:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["nvme", "ns-rescan", "/dev/nvme1"]
  and .partialExecutionRecovery.retryReviewActionIds == ["nvmenamespaces:/dev/nvme1:grow"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["nvme", "list-ns", "/dev/nvme1", "--all", "--output-format=json"]))
    and (.commands | any(.argv == ["nvme", "list-subsys", "--output-format=json"]))
    and (.notes | any(contains("NVMe namespace changes")))
    and (.notes | any(contains("grow/rescan")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["nvme", "list-ns", "/dev/nvme1", "--all", "--output-format=json"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["nvme", "list-subsys", "--output-format=json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$nvme_grow_json" >/dev/null

cmp "$nvme_grow_json" "$nvme_grow_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "nvmenamespaces:/dev/nvme1:grow"
  and .report.partialExecutionRecovery.failedCommand == ["nvme", "ns-rescan", "/dev/nvme1"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$nvme_grow_receipt" >/dev/null

nvme_attach_tools="$tmpdir/fake-nvme-attach-tools"
mkdir -p "$nvme_attach_tools"

cat > "$nvme_attach_tools/nvme" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "attach-ns" ]]; then
  echo "synthetic nvme namespace attach failure for disk-nix recovery coverage" >&2
  exit 82
fi
printf '{}\n'
EOF

chmod +x "$nvme_attach_tools/nvme"

nvme_attach_spec="$tmpdir/nvme-attach-spec.json"
nvme_attach_json="$tmpdir/nvme-attach-apply.json"
nvme_attach_report="$tmpdir/nvme-attach-report.json"
nvme_attach_receipt="$tmpdir/nvme-attach-receipt.json"

jq -n '{
  nvmeNamespaces: {
    "/dev/nvme2": {
      operation: "attach",
      namespaceId: "7",
      controllers: "0x2"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$nvme_attach_spec"

if PATH="$nvme_attach_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$nvme_attach_spec" \
  --execute \
  --report-out "$nvme_attach_report" \
  --receipt-out "$nvme_attach_receipt" \
  --json > "$nvme_attach_json"; then
  echo "expected synthetic NVMe namespace attach failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 6
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["nvme", "list-ns", "/dev/nvme2", "--all", "--output-format=json"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["nvme", "list-subsys", "--output-format=json"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 82
  and .executionResults[2].argv == ["nvme", "attach-ns", "/dev/nvme2", "--namespace-id", "7", "--controllers", "0x2"]
  and (.executionResults[2].stderr | contains("synthetic nvme namespace attach failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "nvmenamespaces:/dev/nvme2:attach"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["nvme", "attach-ns", "/dev/nvme2", "--namespace-id", "7", "--controllers", "0x2"]
  and .partialExecutionRecovery.retryReviewActionIds == ["nvmenamespaces:/dev/nvme2:attach"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["nvme", "list-ns", "/dev/nvme2", "--all", "--output-format=json"]))
    and (.commands | any(.argv == ["nvme", "list-subsys", "--output-format=json"]))
    and (.notes | any(contains("NVMe namespace changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["nvme", "list-ns", "/dev/nvme2", "--all", "--output-format=json"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["nvme", "list-subsys", "--output-format=json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$nvme_attach_json" >/dev/null

cmp "$nvme_attach_json" "$nvme_attach_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "nvmenamespaces:/dev/nvme2:attach"
  and .report.partialExecutionRecovery.failedCommand == ["nvme", "attach-ns", "/dev/nvme2", "--namespace-id", "7", "--controllers", "0x2"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$nvme_attach_receipt" >/dev/null

nvme_detach_tools="$tmpdir/fake-nvme-detach-tools"
mkdir -p "$nvme_detach_tools"

cat > "$nvme_detach_tools/nvme" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "detach-ns" ]]; then
  echo "synthetic nvme namespace detach failure for disk-nix recovery coverage" >&2
  exit 83
fi
printf '{}\n'
EOF

chmod +x "$nvme_detach_tools/nvme"

nvme_detach_spec="$tmpdir/nvme-detach-spec.json"
nvme_detach_json="$tmpdir/nvme-detach-apply.json"
nvme_detach_report="$tmpdir/nvme-detach-report.json"
nvme_detach_receipt="$tmpdir/nvme-detach-receipt.json"

jq -n '{
  nvmeNamespaces: {
    "/dev/nvme3": {
      operation: "detach",
      namespaceId: "8",
      controllers: "0x3"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$nvme_detach_spec"

if PATH="$nvme_detach_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$nvme_detach_spec" \
  --execute \
  --report-out "$nvme_detach_report" \
  --receipt-out "$nvme_detach_receipt" \
  --json > "$nvme_detach_json"; then
  echo "expected synthetic NVMe namespace detach failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 6
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["nvme", "list-ns", "/dev/nvme3", "--all", "--output-format=json"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["nvme", "list-subsys", "--output-format=json"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 83
  and .executionResults[2].argv == ["nvme", "detach-ns", "/dev/nvme3", "--namespace-id", "8", "--controllers", "0x3"]
  and (.executionResults[2].stderr | contains("synthetic nvme namespace detach failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "nvmenamespaces:/dev/nvme3:detach"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["nvme", "detach-ns", "/dev/nvme3", "--namespace-id", "8", "--controllers", "0x3"]
  and .partialExecutionRecovery.retryReviewActionIds == ["nvmenamespaces:/dev/nvme3:detach"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["nvme", "list-ns", "/dev/nvme3", "--all", "--output-format=json"]))
    and (.commands | any(.argv == ["nvme", "list-subsys", "--output-format=json"]))
    and (.notes | any(contains("NVMe namespace changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["nvme", "list-ns", "/dev/nvme3", "--all", "--output-format=json"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["nvme", "list-subsys", "--output-format=json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$nvme_detach_json" >/dev/null

cmp "$nvme_detach_json" "$nvme_detach_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "nvmenamespaces:/dev/nvme3:detach"
  and .report.partialExecutionRecovery.failedCommand == ["nvme", "detach-ns", "/dev/nvme3", "--namespace-id", "8", "--controllers", "0x3"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$nvme_detach_receipt" >/dev/null

nvme_tools="$tmpdir/fake-nvme-tools"
mkdir -p "$nvme_tools"

cat > "$nvme_tools/nvme" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "delete-ns" ]]; then
  echo "synthetic nvme namespace delete failure for disk-nix recovery coverage" >&2
  exit 75
fi
printf '{}\n'
EOF

chmod +x "$nvme_tools/nvme"

nvme_spec="$tmpdir/nvme-spec.json"
nvme_json="$tmpdir/nvme-apply.json"
nvme_report="$tmpdir/nvme-report.json"
nvme_receipt="$tmpdir/nvme-receipt.json"

jq -n '{
  nvmeNamespaces: {
    "/dev/nvme4": {
      destroy: true,
      namespaceId: "9",
      controllers: "0x4"
    }
  },
  apply: {
    allowDestructive: true,
    allowOffline: true
  }
}' > "$nvme_spec"

if PATH="$nvme_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$nvme_spec" \
  --execute \
  --report-out "$nvme_report" \
  --receipt-out "$nvme_receipt" \
  --json > "$nvme_json"; then
  echo "expected synthetic NVMe namespace delete failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 6
  and (.executionResults | length) == 4
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["nvme", "list-ns", "/dev/nvme4", "--all", "--output-format=json"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["nvme", "list-subsys", "--output-format=json"]
  and .executionResults[2].success == true
  and .executionResults[2].argv == ["nvme", "detach-ns", "/dev/nvme4", "--namespace-id", "9", "--controllers", "0x4"]
  and .executionResults[3].success == false
  and .executionResults[3].statusCode == 75
  and .executionResults[3].argv == ["nvme", "delete-ns", "/dev/nvme4", "--namespace-id", "9"]
  and (.executionResults[3].stderr | contains("synthetic nvme namespace delete failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "nvmenamespaces:/dev/nvme4:destroy"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["nvme", "delete-ns", "/dev/nvme4", "--namespace-id", "9"]
  and .partialExecutionRecovery.retryReviewActionIds == ["nvmenamespaces:/dev/nvme4:destroy"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["nvme", "list-ns", "/dev/nvme4", "--all", "--output-format=json"]))
    and (.commands | any(.argv == ["nvme", "list-subsys", "--output-format=json"]))
    and (.notes | any(contains("NVMe namespace changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["nvme", "list-ns", "/dev/nvme4", "--all", "--output-format=json"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["nvme", "list-subsys", "--output-format=json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$nvme_json" >/dev/null

cmp "$nvme_json" "$nvme_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "nvmenamespaces:/dev/nvme4:destroy"
  and .report.partialExecutionRecovery.failedCommand == ["nvme", "delete-ns", "/dev/nvme4", "--namespace-id", "9"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$nvme_receipt" >/dev/null

target_lun_lio_tools="$tmpdir/fake-target-lun-lio-tools"
mkdir -p "$target_lun_lio_tools"

cat > "$target_lun_lio_tools/targetcli" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns create /backstores/block/_dev_zvol_tank_root lun=7" ]]; then
  echo "synthetic target-side LUN LIO create failure for disk-nix recovery coverage" >&2
  exit 85
fi
printf '{}\n'
EOF

chmod +x "$target_lun_lio_tools/targetcli"

target_lun_lio_spec="$tmpdir/target-lun-lio-spec.json"
target_lun_lio_json="$tmpdir/target-lun-lio-apply.json"
target_lun_lio_report="$tmpdir/target-lun-lio-report.json"
target_lun_lio_receipt="$tmpdir/target-lun-lio-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:storage.root": {
        operation: "create",
        provider: "lio",
        source: "/dev/zvol/tank/root",
        lun: 7,
        portal: "192.0.2.10:3260",
        client: "iqn.2026-06.example:host.primary"
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$target_lun_lio_spec"

if PATH="$target_lun_lio_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_lio_spec" \
  --execute \
  --report-out "$target_lun_lio_report" \
  --receipt-out "$target_lun_lio_receipt" \
  --json > "$target_lun_lio_json"; then
  echo "expected synthetic target-side LUN LIO create failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 7
  and (.executionResults | length) == 4
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["targetcli", "/iscsi", "ls"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["targetcli", "/backstores/block", "create", "name=_dev_zvol_tank_root", "dev=/dev/zvol/tank/root"]
  and .executionResults[2].success == true
  and .executionResults[2].argv == ["targetcli", "/iscsi", "create", "iqn.2026-06.example:storage.root"]
  and .executionResults[3].success == false
  and .executionResults[3].statusCode == 85
  and .executionResults[3].argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns", "create", "/backstores/block/_dev_zvol_tank_root", "lun=7"]
  and (.executionResults[3].stderr | contains("synthetic target-side LUN LIO create failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:storage.root:create"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns", "create", "/backstores/block/_dev_zvol_tank_root", "lun=7"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetluns:iqn.2026-06.example:storage.root:create"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 2
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["targetcli", "/iscsi", "ls"]))
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["multipath", "-ll"]))
    and (.notes | any(contains("target-side LUN changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$target_lun_lio_json" >/dev/null

cmp "$target_lun_lio_json" "$target_lun_lio_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:storage.root:create"
  and .report.partialExecutionRecovery.failedCommand == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns", "create", "/backstores/block/_dev_zvol_tank_root", "lun=7"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 2
' "$target_lun_lio_receipt" >/dev/null

target_lun_lio_attach_tools="$tmpdir/fake-target-lun-lio-attach-tools"
mkdir -p "$target_lun_lio_attach_tools"

cat > "$target_lun_lio_attach_tools/targetcli" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/iscsi/iqn.2026-06.example:storage.root/tpg1/acls create iqn.2026-06.example:host.primary" ]]; then
  echo "synthetic target-side LUN LIO attach ACL failure for disk-nix recovery coverage" >&2
  exit 81
fi
printf '{}\n'
EOF

chmod +x "$target_lun_lio_attach_tools/targetcli"

target_lun_lio_attach_spec="$tmpdir/target-lun-lio-attach-spec.json"
target_lun_lio_attach_json="$tmpdir/target-lun-lio-attach-apply.json"
target_lun_lio_attach_report="$tmpdir/target-lun-lio-attach-report.json"
target_lun_lio_attach_receipt="$tmpdir/target-lun-lio-attach-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:storage.root": {
        operation: "attach",
        provider: "lio",
        source: "/dev/zvol/tank/root",
        lun: 7,
        client: "iqn.2026-06.example:host.primary"
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$target_lun_lio_attach_spec"

if PATH="$target_lun_lio_attach_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_lio_attach_spec" \
  --execute \
  --report-out "$target_lun_lio_attach_report" \
  --receipt-out "$target_lun_lio_attach_receipt" \
  --json > "$target_lun_lio_attach_json"; then
  echo "expected synthetic target-side LUN LIO attach failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 5
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns", "create", "/backstores/block/_dev_zvol_tank_root", "lun=7"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 81
  and .executionResults[2].argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/acls", "create", "iqn.2026-06.example:host.primary"]
  and (.executionResults[2].stderr | contains("synthetic target-side LUN LIO attach ACL failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:storage.root:attach"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/acls", "create", "iqn.2026-06.example:host.primary"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetluns:iqn.2026-06.example:storage.root:attach"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["targetcli", "/iscsi", "ls"]))
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["multipath", "-ll"]))
    and (.notes | any(contains("target-side LUN changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$target_lun_lio_attach_json" >/dev/null

cmp "$target_lun_lio_attach_json" "$target_lun_lio_attach_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:storage.root:attach"
  and .report.partialExecutionRecovery.failedCommand == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/acls", "create", "iqn.2026-06.example:host.primary"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$target_lun_lio_attach_receipt" >/dev/null

target_lun_lio_detach_tools="$tmpdir/fake-target-lun-lio-detach-tools"
mkdir -p "$target_lun_lio_detach_tools"

cat > "$target_lun_lio_detach_tools/targetcli" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns delete 7" ]]; then
  echo "synthetic target-side LUN LIO detach unmap failure for disk-nix recovery coverage" >&2
  exit 79
fi
printf '{}\n'
EOF

chmod +x "$target_lun_lio_detach_tools/targetcli"

target_lun_lio_detach_spec="$tmpdir/target-lun-lio-detach-spec.json"
target_lun_lio_detach_json="$tmpdir/target-lun-lio-detach-apply.json"
target_lun_lio_detach_report="$tmpdir/target-lun-lio-detach-report.json"
target_lun_lio_detach_receipt="$tmpdir/target-lun-lio-detach-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:storage.root": {
        operation: "detach",
        provider: "lio",
        lun: 7,
        client: "iqn.2026-06.example:host.primary"
      }
    }
  },
  apply: {
    allowDestructive: true,
    allowOffline: true,
    allowPotentialDataLoss: true,
    backupVerified: true
  }
}' > "$target_lun_lio_detach_spec"

if PATH="$target_lun_lio_detach_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_lio_detach_spec" \
  --execute \
  --report-out "$target_lun_lio_detach_report" \
  --receipt-out "$target_lun_lio_detach_receipt" \
  --json > "$target_lun_lio_detach_json"; then
  echo "expected synthetic target-side LUN LIO detach failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 5
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/acls", "delete", "iqn.2026-06.example:host.primary"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 79
  and .executionResults[2].argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns", "delete", "7"]
  and (.executionResults[2].stderr | contains("synthetic target-side LUN LIO detach unmap failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:storage.root:detach"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns", "delete", "7"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetluns:iqn.2026-06.example:storage.root:detach"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["targetcli", "/iscsi", "ls"]))
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["multipath", "-ll"]))
    and (.notes | any(contains("target-side LUN changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$target_lun_lio_detach_json" >/dev/null

cmp "$target_lun_lio_detach_json" "$target_lun_lio_detach_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:storage.root:detach"
  and .report.partialExecutionRecovery.failedCommand == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns", "delete", "7"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$target_lun_lio_detach_receipt" >/dev/null

target_lun_lio_destroy_tools="$tmpdir/fake-target-lun-lio-destroy-tools"
mkdir -p "$target_lun_lio_destroy_tools"

cat > "$target_lun_lio_destroy_tools/targetcli" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/backstores/block delete _dev_zvol_tank_root" ]]; then
  echo "synthetic target-side LUN LIO destroy backstore failure for disk-nix recovery coverage" >&2
  exit 83
fi
printf '{}\n'
EOF

chmod +x "$target_lun_lio_destroy_tools/targetcli"

target_lun_lio_destroy_spec="$tmpdir/target-lun-lio-destroy-spec.json"
target_lun_lio_destroy_json="$tmpdir/target-lun-lio-destroy-apply.json"
target_lun_lio_destroy_report="$tmpdir/target-lun-lio-destroy-report.json"
target_lun_lio_destroy_receipt="$tmpdir/target-lun-lio-destroy-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:storage.root": {
        destroy: true,
        provider: "lio",
        source: "/dev/zvol/tank/root",
        lun: 7,
        client: "iqn.2026-06.example:host.primary"
      }
    }
  },
  apply: {
    allowDestructive: true,
    allowOffline: true,
    backupVerified: true
  }
}' > "$target_lun_lio_destroy_spec"

if PATH="$target_lun_lio_destroy_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_lio_destroy_spec" \
  --execute \
  --report-out "$target_lun_lio_destroy_report" \
  --receipt-out "$target_lun_lio_destroy_receipt" \
  --json > "$target_lun_lio_destroy_json"; then
  echo "expected synthetic target-side LUN LIO destroy failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 7
  and (.executionResults | length) == 5
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/acls", "delete", "iqn.2026-06.example:host.primary"]
  and .executionResults[2].success == true
  and .executionResults[2].argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns", "delete", "7"]
  and .executionResults[3].success == true
  and .executionResults[3].argv == ["targetcli", "/iscsi", "delete", "iqn.2026-06.example:storage.root"]
  and .executionResults[4].success == false
  and .executionResults[4].statusCode == 83
  and .executionResults[4].argv == ["targetcli", "/backstores/block", "delete", "_dev_zvol_tank_root"]
  and (.executionResults[4].stderr | contains("synthetic target-side LUN LIO destroy backstore failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:storage.root:destroy"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["targetcli", "/backstores/block", "delete", "_dev_zvol_tank_root"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetluns:iqn.2026-06.example:storage.root:destroy"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 3
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["targetcli", "/iscsi", "ls"]))
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["multipath", "-ll"]))
    and (.notes | any(contains("target-side LUN changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$target_lun_lio_destroy_json" >/dev/null

cmp "$target_lun_lio_destroy_json" "$target_lun_lio_destroy_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:storage.root:destroy"
  and .report.partialExecutionRecovery.failedCommand == ["targetcli", "/backstores/block", "delete", "_dev_zvol_tank_root"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 3
' "$target_lun_lio_destroy_receipt" >/dev/null

target_lun_tgt_tools="$tmpdir/fake-target-lun-tgt-tools"
mkdir -p "$target_lun_tgt_tools"

cat > "$target_lun_tgt_tools/tgtadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--lld iscsi --mode logicalunit --op new --tid 42 --lun 8 --backing-store /dev/zvol/tank/root" ]]; then
  echo "synthetic target-side LUN tgt create failure for disk-nix recovery coverage" >&2
  exit 84
fi
printf '{}\n'
EOF

chmod +x "$target_lun_tgt_tools/tgtadm"

target_lun_tgt_spec="$tmpdir/target-lun-tgt-spec.json"
target_lun_tgt_json="$tmpdir/target-lun-tgt-apply.json"
target_lun_tgt_report="$tmpdir/target-lun-tgt-report.json"
target_lun_tgt_receipt="$tmpdir/target-lun-tgt-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:tgt.root": {
        operation: "create",
        provider: "tgt",
        targetId: 42,
        source: "/dev/zvol/tank/root",
        lun: 8,
        client: "ALL"
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$target_lun_tgt_spec"

if PATH="$target_lun_tgt_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_tgt_spec" \
  --execute \
  --report-out "$target_lun_tgt_report" \
  --receipt-out "$target_lun_tgt_receipt" \
  --json > "$target_lun_tgt_json"; then
  echo "expected synthetic target-side LUN tgt create failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 5
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "new", "--tid", "42", "--targetname", "iqn.2026-06.example:tgt.root"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 84
  and .executionResults[2].argv == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "new", "--tid", "42", "--lun", "8", "--backing-store", "/dev/zvol/tank/root"]
  and (.executionResults[2].stderr | contains("synthetic target-side LUN tgt create failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:tgt.root:create"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "new", "--tid", "42", "--lun", "8", "--backing-store", "/dev/zvol/tank/root"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetluns:iqn.2026-06.example:tgt.root:create"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["targetcli", "/iscsi", "ls"]))
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:tgt.root", "ls"]))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["multipath", "-ll"]))
    and (.notes | any(contains("target-side LUN changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:tgt.root", "ls"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$target_lun_tgt_json" >/dev/null

cmp "$target_lun_tgt_json" "$target_lun_tgt_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:tgt.root:create"
  and .report.partialExecutionRecovery.failedCommand == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "new", "--tid", "42", "--lun", "8", "--backing-store", "/dev/zvol/tank/root"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$target_lun_tgt_receipt" >/dev/null

target_lun_tgt_attach_tools="$tmpdir/fake-target-lun-tgt-attach-tools"
mkdir -p "$target_lun_tgt_attach_tools"

cat > "$target_lun_tgt_attach_tools/tgtadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--lld iscsi --mode target --op bind --tid 42 --initiator-address ALL" ]]; then
  echo "synthetic target-side LUN tgt attach bind failure for disk-nix recovery coverage" >&2
  exit 80
fi
printf '{}\n'
EOF

chmod +x "$target_lun_tgt_attach_tools/tgtadm"

target_lun_tgt_attach_spec="$tmpdir/target-lun-tgt-attach-spec.json"
target_lun_tgt_attach_json="$tmpdir/target-lun-tgt-attach-apply.json"
target_lun_tgt_attach_report="$tmpdir/target-lun-tgt-attach-report.json"
target_lun_tgt_attach_receipt="$tmpdir/target-lun-tgt-attach-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:tgt.root": {
        operation: "attach",
        provider: "tgt",
        targetId: 42,
        source: "/dev/zvol/tank/root",
        lun: 8,
        client: "ALL"
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$target_lun_tgt_attach_spec"

if PATH="$target_lun_tgt_attach_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_tgt_attach_spec" \
  --execute \
  --report-out "$target_lun_tgt_attach_report" \
  --receipt-out "$target_lun_tgt_attach_receipt" \
  --json > "$target_lun_tgt_attach_json"; then
  echo "expected synthetic target-side LUN tgt attach failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 4
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "new", "--tid", "42", "--lun", "8", "--backing-store", "/dev/zvol/tank/root"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 80
  and .executionResults[2].argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "bind", "--tid", "42", "--initiator-address", "ALL"]
  and (.executionResults[2].stderr | contains("synthetic target-side LUN tgt attach bind failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:tgt.root:attach"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "bind", "--tid", "42", "--initiator-address", "ALL"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetluns:iqn.2026-06.example:tgt.root:attach"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["targetcli", "/iscsi", "ls"]))
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:tgt.root", "ls"]))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["multipath", "-ll"]))
    and (.notes | any(contains("target-side LUN changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$target_lun_tgt_attach_json" >/dev/null

cmp "$target_lun_tgt_attach_json" "$target_lun_tgt_attach_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:tgt.root:attach"
  and .report.partialExecutionRecovery.failedCommand == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "bind", "--tid", "42", "--initiator-address", "ALL"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$target_lun_tgt_attach_receipt" >/dev/null

target_lun_tgt_detach_tools="$tmpdir/fake-target-lun-tgt-detach-tools"
mkdir -p "$target_lun_tgt_detach_tools"

cat > "$target_lun_tgt_detach_tools/tgtadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--lld iscsi --mode logicalunit --op delete --tid 42 --lun 8" ]]; then
  echo "synthetic target-side LUN tgt detach logicalunit failure for disk-nix recovery coverage" >&2
  exit 78
fi
printf '{}\n'
EOF

chmod +x "$target_lun_tgt_detach_tools/tgtadm"

target_lun_tgt_detach_spec="$tmpdir/target-lun-tgt-detach-spec.json"
target_lun_tgt_detach_json="$tmpdir/target-lun-tgt-detach-apply.json"
target_lun_tgt_detach_report="$tmpdir/target-lun-tgt-detach-report.json"
target_lun_tgt_detach_receipt="$tmpdir/target-lun-tgt-detach-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:tgt.root": {
        operation: "detach",
        provider: "tgt",
        targetId: 42,
        lun: 8,
        client: "ALL"
      }
    }
  },
  apply: {
    allowDestructive: true,
    allowOffline: true,
    allowPotentialDataLoss: true,
    backupVerified: true
  }
}' > "$target_lun_tgt_detach_spec"

if PATH="$target_lun_tgt_detach_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_tgt_detach_spec" \
  --execute \
  --report-out "$target_lun_tgt_detach_report" \
  --receipt-out "$target_lun_tgt_detach_receipt" \
  --json > "$target_lun_tgt_detach_json"; then
  echo "expected synthetic target-side LUN tgt detach failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 4
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "unbind", "--tid", "42", "--initiator-address", "ALL"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 78
  and .executionResults[2].argv == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "delete", "--tid", "42", "--lun", "8"]
  and (.executionResults[2].stderr | contains("synthetic target-side LUN tgt detach logicalunit failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:tgt.root:detach"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "delete", "--tid", "42", "--lun", "8"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetluns:iqn.2026-06.example:tgt.root:detach"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["targetcli", "/iscsi", "ls"]))
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:tgt.root", "ls"]))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["multipath", "-ll"]))
    and (.notes | any(contains("target-side LUN changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$target_lun_tgt_detach_json" >/dev/null

cmp "$target_lun_tgt_detach_json" "$target_lun_tgt_detach_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:tgt.root:detach"
  and .report.partialExecutionRecovery.failedCommand == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "delete", "--tid", "42", "--lun", "8"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$target_lun_tgt_detach_receipt" >/dev/null

target_lun_tgt_destroy_tools="$tmpdir/fake-target-lun-tgt-destroy-tools"
mkdir -p "$target_lun_tgt_destroy_tools"

cat > "$target_lun_tgt_destroy_tools/tgtadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--lld iscsi --mode target --op delete --tid 42" ]]; then
  echo "synthetic target-side LUN tgt destroy target failure for disk-nix recovery coverage" >&2
  exit 82
fi
printf '{}\n'
EOF

chmod +x "$target_lun_tgt_destroy_tools/tgtadm"

target_lun_tgt_destroy_spec="$tmpdir/target-lun-tgt-destroy-spec.json"
target_lun_tgt_destroy_json="$tmpdir/target-lun-tgt-destroy-apply.json"
target_lun_tgt_destroy_report="$tmpdir/target-lun-tgt-destroy-report.json"
target_lun_tgt_destroy_receipt="$tmpdir/target-lun-tgt-destroy-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:tgt.root": {
        destroy: true,
        provider: "tgt",
        targetId: 42,
        lun: 8,
        client: "ALL"
      }
    }
  },
  apply: {
    allowDestructive: true,
    allowOffline: true,
    backupVerified: true
  }
}' > "$target_lun_tgt_destroy_spec"

if PATH="$target_lun_tgt_destroy_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_tgt_destroy_spec" \
  --execute \
  --report-out "$target_lun_tgt_destroy_report" \
  --receipt-out "$target_lun_tgt_destroy_receipt" \
  --json > "$target_lun_tgt_destroy_json"; then
  echo "expected synthetic target-side LUN tgt destroy failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 5
  and (.executionResults | length) == 4
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "unbind", "--tid", "42", "--initiator-address", "ALL"]
  and .executionResults[2].success == true
  and .executionResults[2].argv == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "delete", "--tid", "42", "--lun", "8"]
  and .executionResults[3].success == false
  and .executionResults[3].statusCode == 82
  and .executionResults[3].argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "delete", "--tid", "42"]
  and (.executionResults[3].stderr | contains("synthetic target-side LUN tgt destroy target failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:tgt.root:destroy"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "delete", "--tid", "42"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetluns:iqn.2026-06.example:tgt.root:destroy"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 2
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["targetcli", "/iscsi", "ls"]))
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:tgt.root", "ls"]))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["multipath", "-ll"]))
    and (.notes | any(contains("target-side LUN changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$target_lun_tgt_destroy_json" >/dev/null

cmp "$target_lun_tgt_destroy_json" "$target_lun_tgt_destroy_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:tgt.root:destroy"
  and .report.partialExecutionRecovery.failedCommand == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "delete", "--tid", "42"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 2
' "$target_lun_tgt_destroy_receipt" >/dev/null

multipath_resize_tools="$tmpdir/fake-multipath-resize-tools"
mkdir -p "$multipath_resize_tools"

cat > "$multipath_resize_tools/multipath" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$multipath_resize_tools/lsscsi" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$multipath_resize_tools/multipathd" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "resize map /dev/mapper/mpatha" ]]; then
  echo "synthetic multipath resize failure for disk-nix recovery coverage" >&2
  exit 81
fi
printf '{}\n'
EOF

chmod +x "$multipath_resize_tools/multipath" "$multipath_resize_tools/lsscsi" "$multipath_resize_tools/multipathd"

multipath_resize_spec="$tmpdir/multipath-resize-spec.json"
multipath_resize_json="$tmpdir/multipath-resize-apply.json"
multipath_resize_report="$tmpdir/multipath-resize-report.json"
multipath_resize_receipt="$tmpdir/multipath-resize-receipt.json"

jq -n '{
  multipathMaps: {
    "root-map": {
      device: "/dev/mapper/mpatha",
      operation: "grow"
    }
  },
  apply: {
    allowGrow: true,
    allowOffline: true
  }
}' > "$multipath_resize_spec"

if PATH="$multipath_resize_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$multipath_resize_spec" \
  --execute \
  --report-out "$multipath_resize_report" \
  --receipt-out "$multipath_resize_receipt" \
  --json > "$multipath_resize_json"; then
  echo "expected synthetic multipath resize failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 4
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["multipath", "-ll", "/dev/mapper/mpatha"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["lsscsi", "-t", "-s"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 81
  and .executionResults[2].argv == ["multipathd", "resize", "map", "/dev/mapper/mpatha"]
  and (.executionResults[2].stderr | contains("synthetic multipath resize failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "multipathmaps:root-map:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["multipathd", "resize", "map", "/dev/mapper/mpatha"]
  and .partialExecutionRecovery.retryReviewActionIds == ["multipathmaps:root-map:grow"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["multipath", "-ll", "/dev/mapper/mpatha"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["disk-nix", "multipath", "--json"]))
    and (.notes | any(contains("multipath changes")))
    and (.notes | any(contains("reload, resize")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["multipath", "-ll", "/dev/mapper/mpatha"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["disk-nix", "multipath", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$multipath_resize_json" >/dev/null

cmp "$multipath_resize_json" "$multipath_resize_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "multipathmaps:root-map:grow"
  and .report.partialExecutionRecovery.failedCommand == ["multipathd", "resize", "map", "/dev/mapper/mpatha"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$multipath_resize_receipt" >/dev/null

multipath_replace_tools="$tmpdir/fake-multipath-replace-tools"
mkdir -p "$multipath_replace_tools"

cat > "$multipath_replace_tools/multipath" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$multipath_replace_tools/multipathd" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "del path /dev/sdc" ]]; then
  echo "synthetic multipath replace delete failure for disk-nix recovery coverage" >&2
  exit 87
fi
printf '{}\n'
EOF

cat > "$multipath_replace_tools/lsscsi" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

chmod +x "$multipath_replace_tools/multipath" "$multipath_replace_tools/multipathd" "$multipath_replace_tools/lsscsi"

multipath_replace_spec="$tmpdir/multipath-replace-spec.json"
multipath_replace_json="$tmpdir/multipath-replace-apply.json"
multipath_replace_report="$tmpdir/multipath-replace-report.json"
multipath_replace_receipt="$tmpdir/multipath-replace-receipt.json"

jq -n '{
  spec: {
    multipathMaps: {
      "root-map": {
        device: "/dev/mapper/mpatha",
        replaceDevices: {
          "/dev/sdc": "/dev/sdd"
        }
      }
    }
  },
  apply: {
    allowOffline: true,
    allowDeviceReplacement: true
  }
}' > "$multipath_replace_spec"

if PATH="$multipath_replace_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$multipath_replace_spec" \
  --execute \
  --report-out "$multipath_replace_report" \
  --receipt-out "$multipath_replace_receipt" \
  --json > "$multipath_replace_json"; then
  echo "expected synthetic multipath replace failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 3
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["multipath", "-ll", "/dev/mapper/mpatha"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["multipathd", "add", "path", "/dev/sdd"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 87
  and .executionResults[2].argv == ["multipathd", "del", "path", "/dev/sdc"]
  and (.executionResults[2].stderr | contains("synthetic multipath replace delete failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "multipathMaps:root-map:replace-device:/dev/sdc"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["multipathd", "del", "path", "/dev/sdc"]
  and .partialExecutionRecovery.retryReviewActionIds == ["multipathMaps:root-map:replace-device:/dev/sdc"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["multipath", "-ll", "/dev/mapper/mpatha"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["disk-nix", "multipath", "--json"]))
    and (.notes | any(contains("multipath changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["multipath", "-ll", "/dev/mapper/mpatha"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["disk-nix", "multipath", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$multipath_replace_json" >/dev/null

cmp "$multipath_replace_json" "$multipath_replace_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "multipathMaps:root-map:replace-device:/dev/sdc"
  and .report.partialExecutionRecovery.failedCommand == ["multipathd", "del", "path", "/dev/sdc"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$multipath_replace_receipt" >/dev/null

md_replace_tools="$tmpdir/fake-md-replace-tools"
mkdir -p "$md_replace_tools"

cat > "$md_replace_tools/mdadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/dev/md/root --replace /dev/disk/by-id/old-md-member --with /dev/disk/by-id/new-md-member" ]]; then
  echo "synthetic MD RAID replace failure for disk-nix recovery coverage" >&2
  exit 86
fi
printf '{}\n'
EOF

chmod +x "$md_replace_tools/mdadm"

md_replace_spec="$tmpdir/md-replace-spec.json"
md_replace_json="$tmpdir/md-replace-apply.json"
md_replace_report="$tmpdir/md-replace-report.json"
md_replace_receipt="$tmpdir/md-replace-receipt.json"

jq -n '{
  mdRaids: {
    root: {
      target: "/dev/md/root",
      replaceDevices: {
        "/dev/disk/by-id/old-md-member": "/dev/disk/by-id/new-md-member"
      }
    }
  },
  apply: {
    allowOffline: true,
    allowDeviceReplacement: true
  }
}' > "$md_replace_spec"

if PATH="$md_replace_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$md_replace_spec" \
  --execute \
  --report-out "$md_replace_report" \
  --receipt-out "$md_replace_receipt" \
  --json > "$md_replace_json"; then
  echo "expected synthetic MD RAID replace failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["mdadm", "--detail", "/dev/md/root"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 86
  and .executionResults[1].argv == ["mdadm", "/dev/md/root", "--replace", "/dev/disk/by-id/old-md-member", "--with", "/dev/disk/by-id/new-md-member"]
  and (.executionResults[1].stderr | contains("synthetic MD RAID replace failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "mdRaids:root:replace-device:/dev/disk/by-id/old-md-member"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["mdadm", "/dev/md/root", "--replace", "/dev/disk/by-id/old-md-member", "--with", "/dev/disk/by-id/new-md-member"]
  and .partialExecutionRecovery.retryReviewActionIds == ["mdRaids:root:replace-device:/dev/disk/by-id/old-md-member"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["mdadm", "--detail", "/dev/md/root"]))
    and (.commands | any(.argv == ["cat", "/proc/mdstat"]))
    and (.notes | any(contains("MD RAID member changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["mdadm", "--detail", "/dev/md/root"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["cat", "/proc/mdstat"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$md_replace_json" >/dev/null

cmp "$md_replace_json" "$md_replace_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "mdRaids:root:replace-device:/dev/disk/by-id/old-md-member"
  and .report.partialExecutionRecovery.failedCommand == ["mdadm", "/dev/md/root", "--replace", "/dev/disk/by-id/old-md-member", "--with", "/dev/disk/by-id/new-md-member"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$md_replace_receipt" >/dev/null

luks_open_tools="$tmpdir/fake-luks-open-tools"
mkdir -p "$luks_open_tools"

cat > "$luks_open_tools/cryptsetup" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "open /dev/disk/by-id/archive-luks cryptarchive" ]]; then
  echo "synthetic LUKS open failure for disk-nix recovery coverage" >&2
  exit 83
fi
printf '{}\n'
EOF

chmod +x "$luks_open_tools/cryptsetup"

luks_open_spec="$tmpdir/luks-open-spec.json"
luks_open_json="$tmpdir/luks-open-apply.json"
luks_open_report="$tmpdir/luks-open-report.json"
luks_open_receipt="$tmpdir/luks-open-receipt.json"

jq -n '{
  luks: {
    devices: {
      cryptarchive: {
        name: "cryptarchive",
        device: "/dev/disk/by-id/archive-luks",
        operation: "open"
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$luks_open_spec"

if PATH="$luks_open_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$luks_open_spec" \
  --execute \
  --report-out "$luks_open_report" \
  --receipt-out "$luks_open_receipt" \
  --json > "$luks_open_json"; then
  echo "expected synthetic LUKS open failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 3
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["disk-nix", "inspect", "/dev/disk/by-id/archive-luks"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["cryptsetup", "isLuks", "/dev/disk/by-id/archive-luks"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 83
  and .executionResults[2].argv == ["cryptsetup", "open", "/dev/disk/by-id/archive-luks", "cryptarchive"]
  and (.executionResults[2].stderr | contains("synthetic LUKS open failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "luks.devices:cryptarchive:open"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["cryptsetup", "open", "/dev/disk/by-id/archive-luks", "cryptarchive"]
  and .partialExecutionRecovery.retryReviewActionIds == ["luks.devices:cryptarchive:open"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/archive-luks"]))
    and (.commands | any(.argv == ["cryptsetup", "status", "cryptarchive"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/archive-luks", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "cryptarchive", "--json"]))
    and (.notes | any(contains("LUKS changes")))
    and (.notes | any(contains("alternate unlock paths")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["cryptsetup", "status", "cryptarchive"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/archive-luks"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$luks_open_json" >/dev/null

cmp "$luks_open_json" "$luks_open_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "luks.devices:cryptarchive:open"
  and .report.partialExecutionRecovery.failedCommand == ["cryptsetup", "open", "/dev/disk/by-id/archive-luks", "cryptarchive"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$luks_open_receipt" >/dev/null

luks_close_tools="$tmpdir/fake-luks-close-tools"
mkdir -p "$luks_close_tools"

cat > "$luks_close_tools/cryptsetup" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "close cryptclosed" ]]; then
  echo "synthetic LUKS close failure for disk-nix recovery coverage" >&2
  exit 84
fi
printf '{}\n'
EOF

chmod +x "$luks_close_tools/cryptsetup"

luks_close_spec="$tmpdir/luks-close-spec.json"
luks_close_json="$tmpdir/luks-close-apply.json"
luks_close_report="$tmpdir/luks-close-report.json"
luks_close_receipt="$tmpdir/luks-close-receipt.json"

jq -n '{
  luks: {
    devices: {
      cryptclosed: {
        name: "cryptclosed",
        device: "/dev/disk/by-id/closed-luks",
        operation: "close"
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$luks_close_spec"

if PATH="$luks_close_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$luks_close_spec" \
  --execute \
  --report-out "$luks_close_report" \
  --receipt-out "$luks_close_receipt" \
  --json > "$luks_close_json"; then
  echo "expected synthetic LUKS close failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["cryptsetup", "status", "cryptclosed"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 84
  and .executionResults[1].argv == ["cryptsetup", "close", "cryptclosed"]
  and (.executionResults[1].stderr | contains("synthetic LUKS close failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "luks.devices:cryptclosed:close"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["cryptsetup", "close", "cryptclosed"]
  and .partialExecutionRecovery.retryReviewActionIds == ["luks.devices:cryptclosed:close"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["cryptsetup", "status", "cryptclosed"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "cryptclosed", "--json"]))
    and (.notes | any(contains("LUKS changes")))
    and (.notes | any(contains("alternate unlock paths")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["cryptsetup", "status", "cryptclosed"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["cryptsetup", "status", "cryptclosed"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "cryptclosed", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$luks_close_json" >/dev/null

cmp "$luks_close_json" "$luks_close_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "luks.devices:cryptclosed:close"
  and .report.partialExecutionRecovery.failedCommand == ["cryptsetup", "close", "cryptclosed"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$luks_close_receipt" >/dev/null

luks_keyslot_add_tools="$tmpdir/fake-luks-keyslot-add-tools"
mkdir -p "$luks_keyslot_add_tools"

cat > "$luks_keyslot_add_tools/cryptsetup" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "luksAddKey --key-slot 1 /dev/disk/by-id/root-luks /run/keys/root-new" ]]; then
  echo "synthetic LUKS keyslot add failure for disk-nix recovery coverage" >&2
  exit 85
fi
printf '{}\n'
EOF

chmod +x "$luks_keyslot_add_tools/cryptsetup"

luks_keyslot_add_spec="$tmpdir/luks-keyslot-add-spec.json"
luks_keyslot_add_json="$tmpdir/luks-keyslot-add-apply.json"
luks_keyslot_add_report="$tmpdir/luks-keyslot-add-report.json"
luks_keyslot_add_receipt="$tmpdir/luks-keyslot-add-receipt.json"

jq -n '{
  luksKeyslots: {
    "cryptroot:1": {
      operation: "add-key",
      device: "/dev/disk/by-id/root-luks",
      metadata: {
        keySlot: "1",
        newKeyFile: "/run/keys/root-new"
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$luks_keyslot_add_spec"

if PATH="$luks_keyslot_add_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$luks_keyslot_add_spec" \
  --execute \
  --report-out "$luks_keyslot_add_report" \
  --receipt-out "$luks_keyslot_add_receipt" \
  --json > "$luks_keyslot_add_json"; then
  echo "expected synthetic LUKS keyslot add failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 85
  and .executionResults[1].argv == ["cryptsetup", "luksAddKey", "--key-slot", "1", "/dev/disk/by-id/root-luks", "/run/keys/root-new"]
  and (.executionResults[1].stderr | contains("synthetic LUKS keyslot add failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "lukskeyslots:cryptroot:1:add-key"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["cryptsetup", "luksAddKey", "--key-slot", "1", "/dev/disk/by-id/root-luks", "/run/keys/root-new"]
  and .partialExecutionRecovery.retryReviewActionIds == ["lukskeyslots:cryptroot:1:add-key"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/root-luks", "--json"]))
    and (.notes | any(contains("LUKS changes")))
    and (.notes | any(contains("keyslots")))
    and (.notes | any(contains("alternate unlock paths")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/root-luks", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/root-luks", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$luks_keyslot_add_json" >/dev/null

cmp "$luks_keyslot_add_json" "$luks_keyslot_add_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "lukskeyslots:cryptroot:1:add-key"
  and .report.partialExecutionRecovery.failedCommand == ["cryptsetup", "luksAddKey", "--key-slot", "1", "/dev/disk/by-id/root-luks", "/run/keys/root-new"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$luks_keyslot_add_receipt" >/dev/null

luks_token_import_tools="$tmpdir/fake-luks-token-import-tools"
mkdir -p "$luks_token_import_tools"

cat > "$luks_token_import_tools/cryptsetup" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "token import --token-id 0 --json-file /run/keys/root-token.json /dev/disk/by-id/root-luks" ]]; then
  echo "synthetic LUKS token import failure for disk-nix recovery coverage" >&2
  exit 86
fi
printf '{}\n'
EOF

chmod +x "$luks_token_import_tools/cryptsetup"

luks_token_import_spec="$tmpdir/luks-token-import-spec.json"
luks_token_import_json="$tmpdir/luks-token-import-apply.json"
luks_token_import_report="$tmpdir/luks-token-import-report.json"
luks_token_import_receipt="$tmpdir/luks-token-import-receipt.json"

jq -n '{
  luksTokens: {
    "cryptroot:0": {
      operation: "import-token",
      device: "/dev/disk/by-id/root-luks",
      metadata: {
        tokenId: "0",
        tokenFile: "/run/keys/root-token.json"
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$luks_token_import_spec"

if PATH="$luks_token_import_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$luks_token_import_spec" \
  --execute \
  --report-out "$luks_token_import_report" \
  --receipt-out "$luks_token_import_receipt" \
  --json > "$luks_token_import_json"; then
  echo "expected synthetic LUKS token import failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 86
  and .executionResults[1].argv == ["cryptsetup", "token", "import", "--token-id", "0", "--json-file", "/run/keys/root-token.json", "/dev/disk/by-id/root-luks"]
  and (.executionResults[1].stderr | contains("synthetic LUKS token import failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "lukstokens:cryptroot:0:import-token"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["cryptsetup", "token", "import", "--token-id", "0", "--json-file", "/run/keys/root-token.json", "/dev/disk/by-id/root-luks"]
  and .partialExecutionRecovery.retryReviewActionIds == ["lukstokens:cryptroot:0:import-token"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/root-luks", "--json"]))
    and (.notes | any(contains("LUKS changes")))
    and (.notes | any(contains("tokens")))
    and (.notes | any(contains("alternate unlock paths")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/root-luks", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/root-luks", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$luks_token_import_json" >/dev/null

cmp "$luks_token_import_json" "$luks_token_import_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "lukstokens:cryptroot:0:import-token"
  and .report.partialExecutionRecovery.failedCommand == ["cryptsetup", "token", "import", "--token-id", "0", "--json-file", "/run/keys/root-token.json", "/dev/disk/by-id/root-luks"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$luks_token_import_receipt" >/dev/null

luks_keyslot_remove_tools="$tmpdir/fake-luks-keyslot-remove-tools"
mkdir -p "$luks_keyslot_remove_tools"

cat > "$luks_keyslot_remove_tools/cryptsetup" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "luksKillSlot /dev/disk/by-id/root-luks 6" ]]; then
  echo "synthetic LUKS keyslot remove failure for disk-nix recovery coverage" >&2
  exit 87
fi
printf '{}\n'
EOF

chmod +x "$luks_keyslot_remove_tools/cryptsetup"

luks_keyslot_remove_spec="$tmpdir/luks-keyslot-remove-spec.json"
luks_keyslot_remove_json="$tmpdir/luks-keyslot-remove-apply.json"
luks_keyslot_remove_report="$tmpdir/luks-keyslot-remove-report.json"
luks_keyslot_remove_receipt="$tmpdir/luks-keyslot-remove-receipt.json"

jq -n '{
  luksKeyslots: {
    rootRemove: {
      operation: "remove-key",
      device: "/dev/disk/by-id/root-luks",
      slot: "6"
    }
  },
  apply: {
    allowOffline: true,
    allowPotentialDataLoss: true,
    requireBackup: false,
    requireConfirmation: false
  }
}' > "$luks_keyslot_remove_spec"

if PATH="$luks_keyslot_remove_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$luks_keyslot_remove_spec" \
  --execute \
  --report-out "$luks_keyslot_remove_report" \
  --receipt-out "$luks_keyslot_remove_receipt" \
  --json > "$luks_keyslot_remove_json"; then
  echo "expected synthetic LUKS keyslot remove failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 87
  and .executionResults[1].argv == ["cryptsetup", "luksKillSlot", "/dev/disk/by-id/root-luks", "6"]
  and (.executionResults[1].stderr | contains("synthetic LUKS keyslot remove failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "lukskeyslots:rootremove:remove-key"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["cryptsetup", "luksKillSlot", "/dev/disk/by-id/root-luks", "6"]
  and .partialExecutionRecovery.retryReviewActionIds == ["lukskeyslots:rootremove:remove-key"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/root-luks", "--json"]))
    and (.notes | any(contains("LUKS changes")))
    and (.notes | any(contains("keyslots")))
    and (.notes | any(contains("alternate unlock paths")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/root-luks", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/root-luks", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$luks_keyslot_remove_json" >/dev/null

cmp "$luks_keyslot_remove_json" "$luks_keyslot_remove_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "lukskeyslots:rootremove:remove-key"
  and .report.partialExecutionRecovery.failedCommand == ["cryptsetup", "luksKillSlot", "/dev/disk/by-id/root-luks", "6"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$luks_keyslot_remove_receipt" >/dev/null

partition_grow_tools="$tmpdir/fake-partition-grow-tools"
mkdir -p "$partition_grow_tools"

cat > "$partition_grow_tools/parted" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "-s /dev/disk/by-id/nvme-root resizepart 2 100%" ]]; then
  echo "synthetic partition grow failure for disk-nix recovery coverage" >&2
  exit 81
fi
printf '{}\n'
EOF

cat > "$partition_grow_tools/partprobe" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$partition_grow_tools/blockdev" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

chmod +x "$partition_grow_tools/parted" "$partition_grow_tools/partprobe" "$partition_grow_tools/blockdev"

partition_grow_spec="$tmpdir/partition-grow-spec.json"
partition_grow_json="$tmpdir/partition-grow-apply.json"
partition_grow_report="$tmpdir/partition-grow-report.json"
partition_grow_receipt="$tmpdir/partition-grow-receipt.json"

jq -n '{
  partitions: {
    root: {
      operation: "grow",
      target: "/dev/disk/by-id/nvme-root-part2",
      device: "/dev/disk/by-id/nvme-root",
      partitionNumber: 2,
      end: "100%"
    }
  },
  apply: {
    allowOffline: true,
    allowGrow: true
  }
}' > "$partition_grow_spec"

if PATH="$partition_grow_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$partition_grow_spec" \
  --execute \
  --report-out "$partition_grow_report" \
  --receipt-out "$partition_grow_receipt" \
  --json > "$partition_grow_json"; then
  echo "expected synthetic partition grow failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 4
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["disk-nix", "inspect", "/dev/disk/by-id/nvme-root-part2"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 81
  and .executionResults[1].argv == ["parted", "-s", "/dev/disk/by-id/nvme-root", "resizepart", "2", "100%"]
  and (.executionResults[1].stderr | contains("synthetic partition grow failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "partitions:root:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["parted", "-s", "/dev/disk/by-id/nvme-root", "resizepart", "2", "100%"]
  and .partialExecutionRecovery.retryReviewActionIds == ["partitions:root:grow"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["parted", "-lm", "/dev/disk/by-id/nvme-root"]))
    and (.commands | any(.argv == ["lsblk", "--json", "--bytes", "--output-all", "/dev/disk/by-id/nvme-root"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/nvme-root", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/nvme-root-part2", "--json"]))
    and (.notes | any(contains("partition-table changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["parted", "-lm", "/dev/disk/by-id/nvme-root"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["lsblk", "--json", "--bytes", "--output-all", "/dev/disk/by-id/nvme-root"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$partition_grow_json" >/dev/null

cmp "$partition_grow_json" "$partition_grow_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "partitions:root:grow"
  and .report.partialExecutionRecovery.failedCommand == ["parted", "-s", "/dev/disk/by-id/nvme-root", "resizepart", "2", "100%"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$partition_grow_receipt" >/dev/null

nfs_remount_tools="$tmpdir/fake-nfs-remount-tools"
mkdir -p "$nfs_remount_tools"

cat > "$nfs_remount_tools/findmnt" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$nfs_remount_tools/nfsstat" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$nfs_remount_tools/mount" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "-o remount,_netdev,ro,vers=4.2 /srv/tuned" ]]; then
  echo "synthetic NFS remount failure for disk-nix recovery coverage" >&2
  exit 80
fi
printf '{}\n'
EOF

chmod +x "$nfs_remount_tools/findmnt" "$nfs_remount_tools/nfsstat" "$nfs_remount_tools/mount"

nfs_remount_spec="$tmpdir/nfs-remount-spec.json"
nfs_remount_json="$tmpdir/nfs-remount-apply.json"
nfs_remount_report="$tmpdir/nfs-remount-report.json"
nfs_remount_receipt="$tmpdir/nfs-remount-receipt.json"

jq -n '{
  nfs: {
    mounts: {
      "/srv/tuned": {
        operation: "remount",
        source: "nas.example.com:/srv/tuned",
        options: ["_netdev", "ro", "vers=4.2"]
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$nfs_remount_spec"

if PATH="$nfs_remount_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$nfs_remount_spec" \
  --execute \
  --report-out "$nfs_remount_report" \
  --receipt-out "$nfs_remount_receipt" \
  --json > "$nfs_remount_json"; then
  echo "expected synthetic NFS remount failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["findmnt", "--json", "/srv/tuned"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 80
  and .executionResults[1].argv == ["mount", "-o", "remount,_netdev,ro,vers=4.2", "/srv/tuned"]
  and (.executionResults[1].stderr | contains("synthetic NFS remount failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "nfs.mounts:/srv/tuned:remount"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["mount", "-o", "remount,_netdev,ro,vers=4.2", "/srv/tuned"]
  and .partialExecutionRecovery.retryReviewActionIds == ["nfs.mounts:/srv/tuned:remount"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["findmnt", "--json", "/srv/tuned"]))
    and (.commands | any(.argv == ["nfsstat", "-m", "/srv/tuned"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/srv/tuned", "--json"]))
    and (.notes | any(contains("NFS changes")))
    and (.notes | any(contains("negotiated mount options")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["nfsstat", "-m", "/srv/tuned"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["findmnt", "--json", "/srv/tuned"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$nfs_remount_json" >/dev/null

cmp "$nfs_remount_json" "$nfs_remount_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "nfs.mounts:/srv/tuned:remount"
  and .report.partialExecutionRecovery.failedCommand == ["mount", "-o", "remount,_netdev,ro,vers=4.2", "/srv/tuned"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$nfs_remount_receipt" >/dev/null

iscsi_tools="$tmpdir/fake-iscsi-tools"
mkdir -p "$iscsi_tools"

cat > "$iscsi_tools/iscsiadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == *"--logout"* ]]; then
  echo "synthetic iscsi logout failure for disk-nix recovery coverage" >&2
  exit 76
fi
printf '{}\n'
EOF

chmod +x "$iscsi_tools/iscsiadm"

iscsi_spec="$tmpdir/iscsi-spec.json"
iscsi_json="$tmpdir/iscsi-apply.json"
iscsi_report="$tmpdir/iscsi-report.json"
iscsi_receipt="$tmpdir/iscsi-receipt.json"

jq -n '{
  iscsiSessions: {
    "iqn.2026-06.example:storage.old": {
      operation: "logout",
      portal: "192.0.2.11:3260"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$iscsi_spec"

if PATH="$iscsi_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$iscsi_spec" \
  --execute \
  --report-out "$iscsi_report" \
  --receipt-out "$iscsi_receipt" \
  --json > "$iscsi_json"; then
  echo "expected synthetic iSCSI logout failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 1
  and (.executionResults | length) == 1
  and .executionResults[0].success == false
  and .executionResults[0].statusCode == 76
  and .executionResults[0].argv == ["iscsiadm", "--mode", "node", "--targetname", "iqn.2026-06.example:storage.old", "--portal", "192.0.2.11:3260", "--logout"]
  and (.executionResults[0].stderr | contains("synthetic iscsi logout failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "iscsisessions:iqn.2026-06.example:storage.old:logout"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["iscsiadm", "--mode", "node", "--targetname", "iqn.2026-06.example:storage.old", "--portal", "192.0.2.11:3260", "--logout"]
  and .partialExecutionRecovery.retryReviewActionIds == ["iscsisessions:iqn.2026-06.example:storage.old:logout"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["iscsiadm", "--mode", "session"]))
    and (.commands | any(.argv == ["iscsiadm", "--mode", "node", "--targetname", "iqn.2026-06.example:storage.old"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["multipath", "-ll"]))
    and (.notes | any(contains("login or logout")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["iscsiadm", "--mode", "session"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["multipath", "-ll"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$iscsi_json" >/dev/null

cmp "$iscsi_json" "$iscsi_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "iscsisessions:iqn.2026-06.example:storage.old:logout"
  and .report.partialExecutionRecovery.failedCommand == ["iscsiadm", "--mode", "node", "--targetname", "iqn.2026-06.example:storage.old", "--portal", "192.0.2.11:3260", "--logout"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$iscsi_receipt" >/dev/null

iscsi_login_tools="$tmpdir/fake-iscsi-login-tools"
mkdir -p "$iscsi_login_tools"

cat > "$iscsi_login_tools/iscsiadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == *"--login"* ]]; then
  echo "synthetic iscsi login failure for disk-nix recovery coverage" >&2
  exit 78
fi
printf '{}\n'
EOF

chmod +x "$iscsi_login_tools/iscsiadm"

iscsi_login_spec="$tmpdir/iscsi-login-spec.json"
iscsi_login_json="$tmpdir/iscsi-login-apply.json"
iscsi_login_report="$tmpdir/iscsi-login-report.json"
iscsi_login_receipt="$tmpdir/iscsi-login-receipt.json"

jq -n '{
  iscsiSessions: {
    "iqn.2026-06.example:storage.root": {
      operation: "login",
      portal: "192.0.2.10:3260"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$iscsi_login_spec"

if PATH="$iscsi_login_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$iscsi_login_spec" \
  --execute \
  --report-out "$iscsi_login_report" \
  --receipt-out "$iscsi_login_receipt" \
  --json > "$iscsi_login_json"; then
  echo "expected synthetic iSCSI login failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["iscsiadm", "--mode", "discovery", "--type", "sendtargets", "--portal", "192.0.2.10:3260"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 78
  and .executionResults[1].argv == ["iscsiadm", "--mode", "node", "--targetname", "iqn.2026-06.example:storage.root", "--portal", "192.0.2.10:3260", "--login"]
  and (.executionResults[1].stderr | contains("synthetic iscsi login failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "iscsisessions:iqn.2026-06.example:storage.root:login"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["iscsiadm", "--mode", "node", "--targetname", "iqn.2026-06.example:storage.root", "--portal", "192.0.2.10:3260", "--login"]
  and .partialExecutionRecovery.retryReviewActionIds == ["iscsisessions:iqn.2026-06.example:storage.root:login"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["iscsiadm", "--mode", "session"]))
    and (.commands | any(.argv == ["iscsiadm", "--mode", "node", "--targetname", "iqn.2026-06.example:storage.root"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["multipath", "-ll"]))
    and (.notes | any(contains("login or logout")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["iscsiadm", "--mode", "session"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["multipath", "-ll"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$iscsi_login_json" >/dev/null

cmp "$iscsi_login_json" "$iscsi_login_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "iscsisessions:iqn.2026-06.example:storage.root:login"
  and .report.partialExecutionRecovery.failedCommand == ["iscsiadm", "--mode", "node", "--targetname", "iqn.2026-06.example:storage.root", "--portal", "192.0.2.10:3260", "--login"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$iscsi_login_receipt" >/dev/null

lvm_cache_attach_tools="$tmpdir/fake-lvm-cache-attach-tools"
mkdir -p "$lvm_cache_attach_tools"

cat > "$lvm_cache_attach_tools/lvconvert" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == *"--type cache"* ]]; then
  echo "synthetic lvm cache attach failure for disk-nix recovery coverage" >&2
  exit 79
fi
printf '{}\n'
EOF

chmod +x "$lvm_cache_attach_tools/lvconvert"

lvm_cache_attach_spec="$tmpdir/lvm-cache-attach-spec.json"
lvm_cache_attach_json="$tmpdir/lvm-cache-attach-apply.json"
lvm_cache_attach_report="$tmpdir/lvm-cache-attach-report.json"
lvm_cache_attach_receipt="$tmpdir/lvm-cache-attach-receipt.json"

jq -n '{
  lvmCaches: {
    "vg0/root": {
      addDevices: ["vg0/root-cache"]
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$lvm_cache_attach_spec"

if PATH="$lvm_cache_attach_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$lvm_cache_attach_spec" \
  --execute \
  --report-out "$lvm_cache_attach_report" \
  --receipt-out "$lvm_cache_attach_receipt" \
  --json > "$lvm_cache_attach_json"; then
  echo "expected synthetic LVM cache attach failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["disk-nix", "inspect", "vg0/root"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 79
  and .executionResults[1].argv == ["lvconvert", "--type", "cache", "--cachepool", "vg0/root-cache", "vg0/root"]
  and (.executionResults[1].stderr | contains("synthetic lvm cache attach failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "lvmCaches:vg0/root:add-device:vg0/root-cache"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["lvconvert", "--type", "cache", "--cachepool", "vg0/root-cache", "vg0/root"]
  and .partialExecutionRecovery.retryReviewActionIds == ["lvmCaches:vg0/root:add-device:vg0/root-cache"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/root"]))
    and (.commands | any(.argv == ["vgs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "vg0/root", "--json"]))
    and (.notes | any(contains("cache changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/root"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/root"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$lvm_cache_attach_json" >/dev/null

cmp "$lvm_cache_attach_json" "$lvm_cache_attach_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "lvmCaches:vg0/root:add-device:vg0/root-cache"
  and .report.partialExecutionRecovery.failedCommand == ["lvconvert", "--type", "cache", "--cachepool", "vg0/root-cache", "vg0/root"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$lvm_cache_attach_receipt" >/dev/null

lvm_cache_detach_tools="$tmpdir/fake-lvm-cache-detach-tools"
mkdir -p "$lvm_cache_detach_tools"

cat > "$lvm_cache_detach_tools/lvs" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$lvm_cache_detach_tools/lvconvert" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == *"--uncache"* ]]; then
  echo "synthetic lvm cache detach failure for disk-nix recovery coverage" >&2
  exit 80
fi
printf '{}\n'
EOF

chmod +x "$lvm_cache_detach_tools/lvs" "$lvm_cache_detach_tools/lvconvert"

lvm_cache_detach_spec="$tmpdir/lvm-cache-detach-spec.json"
lvm_cache_detach_json="$tmpdir/lvm-cache-detach-apply.json"
lvm_cache_detach_report="$tmpdir/lvm-cache-detach-report.json"
lvm_cache_detach_receipt="$tmpdir/lvm-cache-detach-receipt.json"

jq -n '{
  lvmCaches: {
    "vg0/root": {
      removeDevices: ["vg0/root-cache"]
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$lvm_cache_detach_spec"

if PATH="$lvm_cache_detach_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$lvm_cache_detach_spec" \
  --execute \
  --report-out "$lvm_cache_detach_report" \
  --receipt-out "$lvm_cache_detach_receipt" \
  --json > "$lvm_cache_detach_json"; then
  echo "expected synthetic LVM cache detach failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["lvs", "--reportformat", "json", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent", "vg0/root"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 80
  and .executionResults[1].argv == ["lvconvert", "--uncache", "vg0/root"]
  and (.executionResults[1].stderr | contains("synthetic lvm cache detach failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "lvmCaches:vg0/root:remove-device:vg0/root-cache"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["lvconvert", "--uncache", "vg0/root"]
  and .partialExecutionRecovery.retryReviewActionIds == ["lvmCaches:vg0/root:remove-device:vg0/root-cache"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/root"]))
    and (.commands | any(.argv == ["vgs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "vg0/root", "--json"]))
    and (.notes | any(contains("cache changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/root"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/root"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$lvm_cache_detach_json" >/dev/null

cmp "$lvm_cache_detach_json" "$lvm_cache_detach_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "lvmCaches:vg0/root:remove-device:vg0/root-cache"
  and .report.partialExecutionRecovery.failedCommand == ["lvconvert", "--uncache", "vg0/root"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$lvm_cache_detach_receipt" >/dev/null

vdo_grow_tools="$tmpdir/fake-vdo-grow-tools"
mkdir -p "$vdo_grow_tools"

cat > "$vdo_grow_tools/vdo" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "growLogical" ]]; then
  echo "synthetic VDO grow failure for disk-nix recovery coverage" >&2
  exit 82
fi
printf '{}\n'
EOF

cat > "$vdo_grow_tools/vdostats" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

chmod +x "$vdo_grow_tools/vdo" "$vdo_grow_tools/vdostats"

vdo_grow_spec="$tmpdir/vdo-grow-spec.json"
vdo_grow_json="$tmpdir/vdo-grow-apply.json"
vdo_grow_report="$tmpdir/vdo-grow-report.json"
vdo_grow_receipt="$tmpdir/vdo-grow-receipt.json"

jq -n '{
  vdoVolumes: {
    archive: {
      operation: "grow",
      desiredSize: "4TiB"
    }
  },
  apply: {
    allowGrow: true
  }
}' > "$vdo_grow_spec"

if PATH="$vdo_grow_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$vdo_grow_spec" \
  --execute \
  --report-out "$vdo_grow_report" \
  --receipt-out "$vdo_grow_receipt" \
  --json > "$vdo_grow_json"; then
  echo "expected synthetic VDO grow failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["vdo", "status", "--name", "archive"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 82
  and .executionResults[1].argv == ["vdo", "growLogical", "--name", "archive", "--vdoLogicalSize", "4TiB"]
  and (.executionResults[1].stderr | contains("synthetic VDO grow failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "vdovolumes:archive:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["vdo", "growLogical", "--name", "archive", "--vdoLogicalSize", "4TiB"]
  and .partialExecutionRecovery.retryReviewActionIds == ["vdovolumes:archive:grow"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["vdo", "status", "--name", "archive"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "archive"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
    and (.notes | any(contains("VDO lifecycle changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["vdo", "status", "--name", "archive"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "archive"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$vdo_grow_json" >/dev/null

cmp "$vdo_grow_json" "$vdo_grow_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "vdovolumes:archive:grow"
  and .report.partialExecutionRecovery.failedCommand == ["vdo", "growLogical", "--name", "archive", "--vdoLogicalSize", "4TiB"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$vdo_grow_receipt" >/dev/null

vdo_property_tools="$tmpdir/fake-vdo-property-tools"
mkdir -p "$vdo_property_tools"

cat > "$vdo_property_tools/vdo" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "changeWritePolicy" ]]; then
  echo "synthetic VDO property failure for disk-nix recovery coverage" >&2
  exit 86
fi
printf '{}\n'
EOF

cat > "$vdo_property_tools/vdostats" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

chmod +x "$vdo_property_tools/vdo" "$vdo_property_tools/vdostats"

vdo_property_spec="$tmpdir/vdo-property-spec.json"
vdo_property_json="$tmpdir/vdo-property-apply.json"
vdo_property_report="$tmpdir/vdo-property-report.json"
vdo_property_receipt="$tmpdir/vdo-property-receipt.json"

jq -n '{
  vdoVolumes: {
    archive: {
      properties: {
        writePolicy: "sync"
      }
    }
  },
  apply: {
    allowPropertyChanges: true
  }
}' > "$vdo_property_spec"

if PATH="$vdo_property_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$vdo_property_spec" \
  --execute \
  --report-out "$vdo_property_report" \
  --receipt-out "$vdo_property_receipt" \
  --json > "$vdo_property_json"; then
  echo "expected synthetic VDO property failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["disk-nix", "inspect", "archive"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 86
  and .executionResults[1].argv == ["vdo", "changeWritePolicy", "--name", "archive", "--writePolicy", "sync"]
  and (.executionResults[1].stderr | contains("synthetic VDO property failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "vdoVolumes:archive:set-property:writePolicy"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["vdo", "changeWritePolicy", "--name", "archive", "--writePolicy", "sync"]
  and .partialExecutionRecovery.retryReviewActionIds == ["vdoVolumes:archive:set-property:writePolicy"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["vdo", "status", "--name", "archive"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "archive"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
    and (.notes | any(contains("VDO lifecycle changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["vdo", "status", "--name", "archive"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "archive"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$vdo_property_json" >/dev/null

cmp "$vdo_property_json" "$vdo_property_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "vdoVolumes:archive:set-property:writePolicy"
  and .report.partialExecutionRecovery.failedCommand == ["vdo", "changeWritePolicy", "--name", "archive", "--writePolicy", "sync"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$vdo_property_receipt" >/dev/null

bcache_property_tools="$tmpdir/fake-bcache-property-tools"
mkdir -p "$bcache_property_tools"
bcache_property_disk_nix="$(command -v "$disk_nix_bin")"
bcache_property_real_sh="$(command -v sh)"

cat > "$bcache_property_tools/sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" == "$bcache_property_real_sh" || "\${1:-}" == "/bin/sh" ]]; then
  shift
fi
case "\$*" in
*"command -v"*)
  exit 0
  ;;
*"disk-nix-bcache-property /dev/bcache1 writearound cache_mode"*)
  echo "synthetic bcache property failure for disk-nix recovery coverage" >&2
  exit 78
  ;;
esac
exec "$bcache_property_real_sh" "\$@"
EOF

cat > "$bcache_property_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$bcache_property_disk_nix" "\$@"
EOF

chmod +x "$bcache_property_tools/sh" "$bcache_property_tools/disk-nix"

bcache_property_spec="$tmpdir/bcache-property-spec.json"
bcache_property_json="$tmpdir/bcache-property-apply.json"
bcache_property_report="$tmpdir/bcache-property-report.json"
bcache_property_receipt="$tmpdir/bcache-property-receipt.json"

jq -n '{
  caches: {
    "writeback-cache": {
      path: "/dev/bcache1",
      properties: {
        "bcache.cache-mode": "writearound"
      }
    }
  },
  apply: {
    allowPropertyChanges: true
  }
}' > "$bcache_property_spec"

if PATH="$bcache_property_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$bcache_property_spec" \
  --execute \
  --report-out "$bcache_property_report" \
  --receipt-out "$bcache_property_receipt" \
  --json > "$bcache_property_json"; then
  echo "expected synthetic bcache property failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["disk-nix", "inspect", "/dev/bcache1"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 78
  and .executionResults[1].argv == ["sh", "-c", "printf '\''%s\\n'\'' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"", "disk-nix-bcache-property", "/dev/bcache1", "writearound", "cache_mode"]
  and (.executionResults[1].stderr | contains("synthetic bcache property failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "caches:writeback-cache:set-property:bcache.cache-mode"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["sh", "-c", "printf '\''%s\\n'\'' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"", "disk-nix-bcache-property", "/dev/bcache1", "writearound", "cache_mode"]
  and .partialExecutionRecovery.retryReviewActionIds == ["caches:writeback-cache:set-property:bcache.cache-mode"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache1", "state"]))
    and (.commands | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache1", "dirty_data"]))
    and (.commands | any(.argv == ["disk-nix", "cache", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/bcache1", "--json"]))
    and (.notes | any(contains("cache changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache1", "cache_mode"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$bcache_property_json" >/dev/null

cmp "$bcache_property_json" "$bcache_property_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "caches:writeback-cache:set-property:bcache.cache-mode"
  and .report.partialExecutionRecovery.failedCommand == ["sh", "-c", "printf '\''%s\\n'\'' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"", "disk-nix-bcache-property", "/dev/bcache1", "writearound", "cache_mode"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$bcache_property_receipt" >/dev/null

lvm_cache_tools="$tmpdir/fake-lvm-cache-tools"
mkdir -p "$lvm_cache_tools"

cat > "$lvm_cache_tools/lvchange" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == *"--cachemode"* ]]; then
  echo "synthetic lvm cache property failure for disk-nix recovery coverage" >&2
  exit 77
fi
printf '{}\n'
EOF

chmod +x "$lvm_cache_tools/lvchange"

lvm_cache_spec="$tmpdir/lvm-cache-spec.json"
lvm_cache_json="$tmpdir/lvm-cache-apply.json"
lvm_cache_report="$tmpdir/lvm-cache-report.json"
lvm_cache_receipt="$tmpdir/lvm-cache-receipt.json"

jq -n '{
  lvmCaches: {
    "vg0/root": {
      properties: {
        "lvm.cache-mode": "writethrough"
      }
    }
  },
  apply: {
    allowPropertyChanges: true
  }
}' > "$lvm_cache_spec"

if PATH="$lvm_cache_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$lvm_cache_spec" \
  --execute \
  --report-out "$lvm_cache_report" \
  --receipt-out "$lvm_cache_receipt" \
  --json > "$lvm_cache_json"; then
  echo "expected synthetic LVM cache property failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["disk-nix", "inspect", "vg0/root"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 77
  and .executionResults[1].argv == ["lvchange", "--cachemode", "writethrough", "vg0/root"]
  and (.executionResults[1].stderr | contains("synthetic lvm cache property failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "lvmCaches:vg0/root:set-property:lvm.cache-mode"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["lvchange", "--cachemode", "writethrough", "vg0/root"]
  and .partialExecutionRecovery.retryReviewActionIds == ["lvmCaches:vg0/root:set-property:lvm.cache-mode"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/root"]))
    and (.commands | any(.argv == ["vgs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "vg0/root", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/root"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$lvm_cache_json" >/dev/null

cmp "$lvm_cache_json" "$lvm_cache_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "lvmCaches:vg0/root:set-property:lvm.cache-mode"
  and .report.partialExecutionRecovery.failedCommand == ["lvchange", "--cachemode", "writethrough", "vg0/root"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$lvm_cache_receipt" >/dev/null

echo "failure-recovery integration smoke test verified partialExecutionRecovery after synthetic resize, LVM grow, XFS grow, Btrfs scrub, Btrfs rebalance, filesystem trim, filesystem check, filesystem repair, swap label, device-mapper rename, ZFS dataset rename, Btrfs snapshot clone, ZFS snapshot clone, LVM VG rename, ZFS rollback, NVMe namespace create, NVMe namespace grow, NVMe namespace attach, NVMe namespace detach, NVMe namespace delete, target-side LUN LIO create, target-side LUN LIO attach, target-side LUN LIO detach, target-side LUN LIO destroy, target-side LUN tgt create, target-side LUN tgt attach, target-side LUN tgt detach, target-side LUN tgt destroy, multipath resize, multipath replace, MD RAID replace, LUKS open, LUKS close, LUKS keyslot add, LUKS token import, LUKS keyslot remove, partition grow, NFS remount, iSCSI logout, iSCSI login, LVM cache attach, LVM cache detach, VDO grow, VDO property, bcache property, and LVM cache property failures"
