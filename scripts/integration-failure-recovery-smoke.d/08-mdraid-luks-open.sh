md_create_tools="$tmpdir/fake-md-create-tools"
mkdir -p "$md_create_tools"

cat > "$md_create_tools/cat" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/proc/mdstat" ]]; then
  printf 'Personalities : [raid1]\nunused devices: <none>\n'
  exit 0
fi
exec /usr/bin/env cat "$@"
EOF

cat > "$md_create_tools/mdadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--create /dev/md/newroot --level 1 --raid-devices 2 /dev/disk/by-id/nvme-a /dev/disk/by-id/nvme-b" ]]; then
  echo "synthetic MD RAID create failure for disk-nix recovery coverage" >&2
  exit 89
fi
printf '{}\n'
EOF

chmod +x "$md_create_tools/cat" "$md_create_tools/mdadm"

md_create_spec="$tmpdir/md-create-spec.json"
md_create_json="$tmpdir/md-create-apply.json"
md_create_report="$tmpdir/md-create-report.json"
md_create_receipt="$tmpdir/md-create-receipt.json"

jq -n '{
  mdRaids: {
    newroot: {
      target: "/dev/md/newroot",
      operation: "create",
      level: "1",
      devices: [
        "/dev/disk/by-id/nvme-a",
        "/dev/disk/by-id/nvme-b"
      ]
    }
  },
  apply: {
    allowDestructive: true,
    backupVerified: true
  }
}' > "$md_create_spec"

if PATH="$md_create_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$md_create_spec" \
  --execute \
  --report-out "$md_create_report" \
  --receipt-out "$md_create_receipt" \
  --json > "$md_create_json"; then
  echo "expected synthetic MD RAID create failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and .commandSummary.mutatingCount == 1
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["cat", "/proc/mdstat"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 89
  and .executionResults[1].argv == ["mdadm", "--create", "/dev/md/newroot", "--level", "1", "--raid-devices", "2", "/dev/disk/by-id/nvme-a", "/dev/disk/by-id/nvme-b"]
  and (.executionResults[1].stderr | contains("synthetic MD RAID create failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "mdraids:newroot:create"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["mdadm", "--create", "/dev/md/newroot", "--level", "1", "--raid-devices", "2", "/dev/disk/by-id/nvme-a", "/dev/disk/by-id/nvme-b"]
  and .partialExecutionRecovery.retryReviewActionIds == ["mdraids:newroot:create"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["disk-nix", "inspect", "newroot", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.notes | any(contains("mdraids:newroot:create")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["mdadm", "--detail", "/dev/md/newroot"]))
    and (.commands | any(.argv == ["cat", "/proc/mdstat"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/md/newroot", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | all(.kind != "rollback-review"))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$md_create_json" >/dev/null

cmp "$md_create_json" "$md_create_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "mdraids:newroot:create"
  and .report.partialExecutionRecovery.failedCommand == ["mdadm", "--create", "/dev/md/newroot", "--level", "1", "--raid-devices", "2", "/dev/disk/by-id/nvme-a", "/dev/disk/by-id/nvme-b"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$md_create_receipt" >/dev/null

md_assemble_tools="$tmpdir/fake-md-assemble-tools"
mkdir -p "$md_assemble_tools"

cat > "$md_assemble_tools/cat" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/proc/mdstat" ]]; then
  printf 'Personalities : [raid1]\nmd_existing : inactive nvme-a[0](S) nvme-b[1](S)\nunused devices: <none>\n'
  exit 0
fi
exec /usr/bin/env cat "$@"
EOF

cat > "$md_assemble_tools/mdadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--assemble /dev/md/existing /dev/disk/by-id/nvme-a /dev/disk/by-id/nvme-b" ]]; then
  echo "synthetic MD RAID assemble failure for disk-nix recovery coverage" >&2
  exit 88
fi
printf '{}\n'
EOF

chmod +x "$md_assemble_tools/cat" "$md_assemble_tools/mdadm"

md_assemble_spec="$tmpdir/md-assemble-spec.json"
md_assemble_json="$tmpdir/md-assemble-apply.json"
md_assemble_report="$tmpdir/md-assemble-report.json"
md_assemble_receipt="$tmpdir/md-assemble-receipt.json"

jq -n '{
  mdRaids: {
    existing: {
      target: "/dev/md/existing",
      operation: "assemble",
      devices: [
        "/dev/disk/by-id/nvme-a",
        "/dev/disk/by-id/nvme-b"
      ]
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$md_assemble_spec"

if PATH="$md_assemble_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$md_assemble_spec" \
  --execute \
  --report-out "$md_assemble_report" \
  --receipt-out "$md_assemble_receipt" \
  --json > "$md_assemble_json"; then
  echo "expected synthetic MD RAID assemble failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and .commandSummary.mutatingCount == 1
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["cat", "/proc/mdstat"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 88
  and .executionResults[1].argv == ["mdadm", "--assemble", "/dev/md/existing", "/dev/disk/by-id/nvme-a", "/dev/disk/by-id/nvme-b"]
  and (.executionResults[1].stderr | contains("synthetic MD RAID assemble failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "mdraids:existing:assemble"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["mdadm", "--assemble", "/dev/md/existing", "/dev/disk/by-id/nvme-a", "/dev/disk/by-id/nvme-b"]
  and .partialExecutionRecovery.retryReviewActionIds == ["mdraids:existing:assemble"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "review-execution-failure"
    and (.notes | any(contains("mdraids:existing:assemble")))
    and (.notes | any(contains("mdadm --assemble /dev/md/existing /dev/disk/by-id/nvme-a /dev/disk/by-id/nvme-b")))
  ))
  and (.recoveryActions | any(
    .kind == "inspect-current-state"
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "resume-after-fix"))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$md_assemble_json" >/dev/null

cmp "$md_assemble_json" "$md_assemble_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "mdraids:existing:assemble"
  and .report.partialExecutionRecovery.failedCommand == ["mdadm", "--assemble", "/dev/md/existing", "/dev/disk/by-id/nvme-a", "/dev/disk/by-id/nvme-b"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$md_assemble_receipt" >/dev/null

md_stop_tools="$tmpdir/fake-md-stop-tools"
mkdir -p "$md_stop_tools"

cat > "$md_stop_tools/mdadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--stop /dev/md/oldroot" ]]; then
  echo "synthetic MD RAID stop failure for disk-nix recovery coverage" >&2
  exit 87
fi
printf '{}\n'
EOF

chmod +x "$md_stop_tools/mdadm"

md_stop_spec="$tmpdir/md-stop-spec.json"
md_stop_json="$tmpdir/md-stop-apply.json"
md_stop_report="$tmpdir/md-stop-report.json"
md_stop_receipt="$tmpdir/md-stop-receipt.json"

jq -n '{
  mdRaids: {
    oldroot: {
      target: "/dev/md/oldroot",
      operation: "stop"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$md_stop_spec"

if PATH="$md_stop_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$md_stop_spec" \
  --execute \
  --report-out "$md_stop_report" \
  --receipt-out "$md_stop_receipt" \
  --json > "$md_stop_json"; then
  echo "expected synthetic MD RAID stop failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and .commandSummary.mutatingCount == 1
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["mdadm", "--detail", "/dev/md/oldroot"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 87
  and .executionResults[1].argv == ["mdadm", "--stop", "/dev/md/oldroot"]
  and (.executionResults[1].stderr | contains("synthetic MD RAID stop failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "mdraids:oldroot:stop"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["mdadm", "--stop", "/dev/md/oldroot"]
  and .partialExecutionRecovery.retryReviewActionIds == ["mdraids:oldroot:stop"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["disk-nix", "inspect", "oldroot", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.notes | any(contains("mdraids:oldroot:stop")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["mdadm", "--detail", "/dev/md/oldroot"]))
    and (.commands | any(.argv == ["cat", "/proc/mdstat"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/md/oldroot", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$md_stop_json" >/dev/null

cmp "$md_stop_json" "$md_stop_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "mdraids:oldroot:stop"
  and .report.partialExecutionRecovery.failedCommand == ["mdadm", "--stop", "/dev/md/oldroot"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$md_stop_receipt" >/dev/null

md_grow_tools="$tmpdir/fake-md-grow-tools"
mkdir -p "$md_grow_tools"

cat > "$md_grow_tools/mdadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--grow /dev/md/root --size max" ]]; then
  echo "synthetic MD RAID grow failure for disk-nix recovery coverage" >&2
  exit 88
fi
printf '{}\n'
EOF

chmod +x "$md_grow_tools/mdadm"

md_grow_spec="$tmpdir/md-grow-spec.json"
md_grow_json="$tmpdir/md-grow-apply.json"
md_grow_report="$tmpdir/md-grow-report.json"
md_grow_receipt="$tmpdir/md-grow-receipt.json"

jq -n '{
  mdRaids: {
    root: {
      target: "/dev/md/root",
      operation: "grow",
      desiredSize: "max"
    }
  },
  apply: {
    allowOffline: true,
    allowGrow: true
  }
}' > "$md_grow_spec"

if PATH="$md_grow_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$md_grow_spec" \
  --execute \
  --report-out "$md_grow_report" \
  --receipt-out "$md_grow_receipt" \
  --json > "$md_grow_json"; then
  echo "expected synthetic MD RAID grow failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 3
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["mdadm", "--detail", "/dev/md/root"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 88
  and .executionResults[1].argv == ["mdadm", "--grow", "/dev/md/root", "--size", "max"]
  and (.executionResults[1].stderr | contains("synthetic MD RAID grow failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "mdraids:root:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["mdadm", "--grow", "/dev/md/root", "--size", "max"]
  and .partialExecutionRecovery.retryReviewActionIds == ["mdraids:root:grow"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "review-execution-failure"
    and (.notes | any(contains("mdraids:root:grow")))
    and (.notes | any(contains("mdadm --grow /dev/md/root --size max")))
  ))
  and (.recoveryActions | any(
    .kind == "inspect-current-state"
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "resume-after-fix"))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$md_grow_json" >/dev/null

cmp "$md_grow_json" "$md_grow_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "mdraids:root:grow"
  and .report.partialExecutionRecovery.failedCommand == ["mdadm", "--grow", "/dev/md/root", "--size", "max"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$md_grow_receipt" >/dev/null

md_add_tools="$tmpdir/fake-md-add-tools"
mkdir -p "$md_add_tools"

cat > "$md_add_tools/mdadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/dev/md/root --add /dev/disk/by-id/nvme-spare" ]]; then
  echo "synthetic MD RAID add-member failure for disk-nix recovery coverage" >&2
  exit 82
fi
printf '{}\n'
EOF

chmod +x "$md_add_tools/mdadm"

md_add_spec="$tmpdir/md-add-spec.json"
md_add_json="$tmpdir/md-add-apply.json"
md_add_report="$tmpdir/md-add-report.json"
md_add_receipt="$tmpdir/md-add-receipt.json"

jq -n '{
  mdRaids: {
    root: {
      target: "/dev/md/root",
      addDevices: ["/dev/disk/by-id/nvme-spare"]
    }
  }
}' > "$md_add_spec"

if PATH="$md_add_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$md_add_spec" \
  --execute \
  --report-out "$md_add_report" \
  --receipt-out "$md_add_receipt" \
  --json > "$md_add_json"; then
  echo "expected synthetic MD RAID add-member failure to fail apply" >&2
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
  and .executionResults[1].statusCode == 82
  and .executionResults[1].argv == ["mdadm", "/dev/md/root", "--add", "/dev/disk/by-id/nvme-spare"]
  and (.executionResults[1].stderr | contains("synthetic MD RAID add-member failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "mdRaids:root:add-device:/dev/disk/by-id/nvme-spare"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["mdadm", "/dev/md/root", "--add", "/dev/disk/by-id/nvme-spare"]
  and .partialExecutionRecovery.retryReviewActionIds == ["mdRaids:root:add-device:/dev/disk/by-id/nvme-spare"]
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
' "$md_add_json" >/dev/null

cmp "$md_add_json" "$md_add_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "mdRaids:root:add-device:/dev/disk/by-id/nvme-spare"
  and .report.partialExecutionRecovery.failedCommand == ["mdadm", "/dev/md/root", "--add", "/dev/disk/by-id/nvme-spare"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$md_add_receipt" >/dev/null

md_remove_tools="$tmpdir/fake-md-remove-tools"
mkdir -p "$md_remove_tools"

cat > "$md_remove_tools/mdadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/dev/md/root --remove /dev/disk/by-id/failed-md-member" ]]; then
  echo "synthetic MD RAID remove-member failure for disk-nix recovery coverage" >&2
  exit 87
fi
printf '{}\n'
EOF

chmod +x "$md_remove_tools/mdadm"

md_remove_spec="$tmpdir/md-remove-spec.json"
md_remove_json="$tmpdir/md-remove-apply.json"
md_remove_report="$tmpdir/md-remove-report.json"
md_remove_receipt="$tmpdir/md-remove-receipt.json"

jq -n '{
  mdRaids: {
    root: {
      target: "/dev/md/root",
      removeDevices: ["/dev/disk/by-id/failed-md-member"]
    }
  },
  apply: {
    allowOffline: true,
    allowDeviceReplacement: true,
    allowPotentialDataLoss: true,
    allowDestructive: true,
    backupVerified: true
  }
}' > "$md_remove_spec"

if PATH="$md_remove_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$md_remove_spec" \
  --execute \
  --report-out "$md_remove_report" \
  --receipt-out "$md_remove_receipt" \
  --json > "$md_remove_json"; then
  echo "expected synthetic MD RAID remove-member failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 3
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["mdadm", "--detail", "/dev/md/root"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["mdadm", "/dev/md/root", "--fail", "/dev/disk/by-id/failed-md-member"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 87
  and .executionResults[2].argv == ["mdadm", "/dev/md/root", "--remove", "/dev/disk/by-id/failed-md-member"]
  and (.executionResults[2].stderr | contains("synthetic MD RAID remove-member failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "mdRaids:root:remove-device:/dev/disk/by-id/failed-md-member"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["mdadm", "/dev/md/root", "--remove", "/dev/disk/by-id/failed-md-member"]
  and .partialExecutionRecovery.retryReviewActionIds == ["mdRaids:root:remove-device:/dev/disk/by-id/failed-md-member"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
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
' "$md_remove_json" >/dev/null

cmp "$md_remove_json" "$md_remove_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "mdRaids:root:remove-device:/dev/disk/by-id/failed-md-member"
  and .report.partialExecutionRecovery.failedCommand == ["mdadm", "/dev/md/root", "--remove", "/dev/disk/by-id/failed-md-member"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$md_remove_receipt" >/dev/null

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

luks_format_tools="$tmpdir/fake-luks-format-tools"
mkdir -p "$luks_format_tools"

cat > "$luks_format_tools/cryptsetup" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "luksFormat /dev/disk/by-id/new-luks" ]]; then
  echo "synthetic LUKS format failure for disk-nix recovery coverage" >&2
  exit 90
fi
printf '{}\n'
EOF

chmod +x "$luks_format_tools/cryptsetup"

luks_format_spec="$tmpdir/luks-format-spec.json"
luks_format_json="$tmpdir/luks-format-apply.json"
luks_format_report="$tmpdir/luks-format-report.json"
luks_format_receipt="$tmpdir/luks-format-receipt.json"

jq -n '{
  luks: {
    devices: {
      cryptnew: {
        name: "cryptnew",
        device: "/dev/disk/by-id/new-luks",
        operation: "format"
      }
    }
  },
  apply: {
    allowOffline: true,
    allowFormat: true,
    allowDestructive: true,
    requireBackup: false,
    requireConfirmation: false
  }
}' > "$luks_format_spec"

if PATH="$luks_format_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$luks_format_spec" \
  --execute \
  --report-out "$luks_format_report" \
  --receipt-out "$luks_format_receipt" \
  --json > "$luks_format_json"; then
  echo "expected synthetic LUKS format failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 3
  and .commandSummary.mutatingCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["disk-nix", "inspect", "/dev/disk/by-id/new-luks"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 90
  and .executionResults[1].argv == ["cryptsetup", "luksFormat", "/dev/disk/by-id/new-luks"]
  and (.executionResults[1].stderr | contains("synthetic LUKS format failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "luks.devices:cryptnew:format"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["cryptsetup", "luksFormat", "/dev/disk/by-id/new-luks"]
  and .partialExecutionRecovery.retryReviewActionIds == ["luks.devices:cryptnew:format"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/new-luks"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/new-luks", "--json"]))
    and (.commands | any(.argv == ["cryptsetup", "status", "cryptnew"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "cryptnew", "--json"]))
    and (.notes | any(contains("LUKS changes")))
    and (.notes | any(contains("alternate unlock paths")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/new-luks"]))
    and (.commands | any(.argv == ["cryptsetup", "isLuks", "cryptnew"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/new-luks"]))
    and (.commands | any(.argv == ["cryptsetup", "status", "cryptnew"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$luks_format_json" >/dev/null

cmp "$luks_format_json" "$luks_format_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "luks.devices:cryptnew:format"
  and .report.partialExecutionRecovery.failedCommand == ["cryptsetup", "luksFormat", "/dev/disk/by-id/new-luks"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$luks_format_receipt" >/dev/null

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
