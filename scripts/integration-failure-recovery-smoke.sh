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

echo "failure-recovery integration smoke test verified partialExecutionRecovery after synthetic resize, ZFS rollback, and NVMe namespace delete failures"
