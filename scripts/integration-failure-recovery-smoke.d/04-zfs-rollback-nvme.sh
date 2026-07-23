zfs_pool_replace_tools="$tmpdir/fake-zfs-pool-replace-tools"
mkdir -p "$zfs_pool_replace_tools"
zfs_pool_replace_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$zfs_pool_replace_tools/zpool" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "replace tank /dev/disk/by-id/old-zfs-vdev /dev/disk/by-id/new-zfs-vdev" ]]; then
  echo "synthetic ZFS pool replacement failure for disk-nix recovery coverage" >&2
  exit 86
fi
printf '{}\n'
EOF

cat > "$zfs_pool_replace_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$zfs_pool_replace_disk_nix" "\$@"
EOF

chmod +x "$zfs_pool_replace_tools/zpool" "$zfs_pool_replace_tools/disk-nix"

zfs_pool_replace_spec="$tmpdir/zfs-pool-replace-spec.json"
zfs_pool_replace_json="$tmpdir/zfs-pool-replace-apply.json"
zfs_pool_replace_report="$tmpdir/zfs-pool-replace-report.json"
zfs_pool_replace_receipt="$tmpdir/zfs-pool-replace-receipt.json"

jq -n '{
  pools: {
    tank: {
      target: "tank",
      replaceDevices: {
        "/dev/disk/by-id/old-zfs-vdev": "/dev/disk/by-id/new-zfs-vdev"
      }
    }
  },
  apply: {
    allowOffline: true,
    allowDeviceReplacement: true
  }
}' > "$zfs_pool_replace_spec"

if PATH="$zfs_pool_replace_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$zfs_pool_replace_spec" \
  --execute \
  --report-out "$zfs_pool_replace_report" \
  --receipt-out "$zfs_pool_replace_receipt" \
  --json > "$zfs_pool_replace_json"; then
  echo "expected synthetic ZFS pool replacement failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 1
  and .commandSummary.commandCount == 2
  and .commandSummary.mutatingCount == 1
  and .commandSummary.manualReviewCount == 1
  and .commandSummary.readyCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "pools:tank:replace-device:/dev/disk/by-id/old-zfs-vdev"
  and .executionResults[0].argv == ["disk-nix", "inspect", "tank"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 86
  and .executionResults[1].actionId == "pools:tank:replace-device:/dev/disk/by-id/old-zfs-vdev"
  and .executionResults[1].argv == ["zpool", "replace", "tank", "/dev/disk/by-id/old-zfs-vdev", "/dev/disk/by-id/new-zfs-vdev"]
  and (.executionResults[1].stderr | contains("synthetic ZFS pool replacement failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "pools:tank:replace-device:/dev/disk/by-id/old-zfs-vdev"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["zpool", "replace", "tank", "/dev/disk/by-id/old-zfs-vdev", "/dev/disk/by-id/new-zfs-vdev"]
  and .partialExecutionRecovery.retryReviewActionIds == ["pools:tank:replace-device:/dev/disk/by-id/old-zfs-vdev"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["zpool", "status", "-P", "tank"]))
    and (.commands | any(.argv == ["zpool", "list", "-H", "-p", "tank"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "tank", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
    and (.notes | any(contains("ZFS changes")))
    and (.notes | any(contains("LUN consumers")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["zpool", "status", "-P", "tank"]))
    and (.commands | any(.argv == ["zpool", "list", "-H", "-p", "tank"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "tank", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["zpool", "status", "-P", "tank"]))
    and (.commands | any(.argv == ["zpool", "list", "-H", "-p", "tank"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "tank", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$zfs_pool_replace_json" >/dev/null

cmp "$zfs_pool_replace_json" "$zfs_pool_replace_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "pools:tank:replace-device:/dev/disk/by-id/old-zfs-vdev"
  and .report.partialExecutionRecovery.failedCommand == ["zpool", "replace", "tank", "/dev/disk/by-id/old-zfs-vdev", "/dev/disk/by-id/new-zfs-vdev"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$zfs_pool_replace_receipt" >/dev/null

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
