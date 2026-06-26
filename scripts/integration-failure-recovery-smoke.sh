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

echo "failure-recovery integration smoke test verified partialExecutionRecovery after synthetic resize, ZFS rollback, NVMe namespace create, NVMe namespace grow, NVMe namespace attach, NVMe namespace detach, NVMe namespace delete, iSCSI logout, iSCSI login, LVM cache attach, LVM cache detach, and LVM cache property failures"
