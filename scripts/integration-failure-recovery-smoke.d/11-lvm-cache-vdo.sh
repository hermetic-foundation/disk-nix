lvm_cache_replace_tools="$tmpdir/fake-lvm-cache-replace-tools"
mkdir -p "$lvm_cache_replace_tools"
lvm_cache_replace_disk_nix="$(command -v "$disk_nix_bin")"
lvm_cache_replace_real_sh="$(command -v sh)"

cat > "$lvm_cache_replace_tools/sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" == "$lvm_cache_replace_real_sh" || "\${1:-}" == "/bin/sh" ]]; then
  shift
fi
case "\$*" in
*"command -v"*)
  exit 0
  ;;
*"disk-nix-lvm-cache-replace vg0/root vg0/root-cache-new"*)
  echo "synthetic LVM cache replacement failure for disk-nix recovery coverage" >&2
  exit 88
  ;;
esac
exec "$lvm_cache_replace_real_sh" "\$@"
EOF

cat > "$lvm_cache_replace_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$lvm_cache_replace_disk_nix" "\$@"
EOF

chmod +x "$lvm_cache_replace_tools/sh" "$lvm_cache_replace_tools/disk-nix"

lvm_cache_replace_spec="$tmpdir/lvm-cache-replace-spec.json"
lvm_cache_replace_json="$tmpdir/lvm-cache-replace-apply.json"
lvm_cache_replace_report="$tmpdir/lvm-cache-replace-report.json"
lvm_cache_replace_receipt="$tmpdir/lvm-cache-replace-receipt.json"

jq -n '{
  lvmCaches: {
    "vg0/root": {
      replaceDevices: {
        "vg0/root-cache": "vg0/root-cache-new"
      }
    }
  },
  apply: {
    allowOffline: true,
    allowDeviceReplacement: true
  }
}' > "$lvm_cache_replace_spec"

if PATH="$lvm_cache_replace_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$lvm_cache_replace_spec" \
  --execute \
  --report-out "$lvm_cache_replace_report" \
  --receipt-out "$lvm_cache_replace_receipt" \
  --json > "$lvm_cache_replace_json"; then
  echo "expected synthetic LVM cache replacement failure to fail apply" >&2
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
  and .executionResults[0].actionId == "lvmCaches:vg0/root:replace-device:vg0/root-cache"
  and .executionResults[0].argv == ["disk-nix", "inspect", "vg0/root"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 88
  and .executionResults[1].actionId == "lvmCaches:vg0/root:replace-device:vg0/root-cache"
  and .executionResults[1].argv == ["sh", "-c", "lvconvert --uncache \"$1\" && lvconvert --type cache --cachepool \"$2\" \"$1\"", "disk-nix-lvm-cache-replace", "vg0/root", "vg0/root-cache-new"]
  and (.executionResults[1].stderr | contains("synthetic LVM cache replacement failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "lvmCaches:vg0/root:replace-device:vg0/root-cache"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["sh", "-c", "lvconvert --uncache \"$1\" && lvconvert --type cache --cachepool \"$2\" \"$1\"", "disk-nix-lvm-cache-replace", "vg0/root", "vg0/root-cache-new"]
  and .partialExecutionRecovery.retryReviewActionIds == ["lvmCaches:vg0/root:replace-device:vg0/root-cache"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/root"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "vg0/root", "--json"]))
    and (.commands | any(.argv == ["vgs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
    and (.notes | any(contains("cache changes")))
    and (.notes | any(contains("dirty-data")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/root"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "vg0/root", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/root"]))
    and (.commands | any(.argv == ["vgs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$lvm_cache_replace_json" >/dev/null

cmp "$lvm_cache_replace_json" "$lvm_cache_replace_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "lvmCaches:vg0/root:replace-device:vg0/root-cache"
  and .report.partialExecutionRecovery.failedCommand == ["sh", "-c", "lvconvert --uncache \"$1\" && lvconvert --type cache --cachepool \"$2\" \"$1\"", "disk-nix-lvm-cache-replace", "vg0/root", "vg0/root-cache-new"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$lvm_cache_replace_receipt" >/dev/null

lvm_cache_rescan_tools="$tmpdir/fake-lvm-cache-rescan-tools"
mkdir -p "$lvm_cache_rescan_tools"

cat > "$lvm_cache_rescan_tools/lvs" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == *"vg0/archive"* ]]; then
  echo "synthetic lvm cache rescan failure for disk-nix recovery coverage" >&2
  exit 92
fi
printf '{}\n'
EOF

chmod +x "$lvm_cache_rescan_tools/lvs"

lvm_cache_rescan_spec="$tmpdir/lvm-cache-rescan-spec.json"
lvm_cache_rescan_json="$tmpdir/lvm-cache-rescan-apply.json"
lvm_cache_rescan_report="$tmpdir/lvm-cache-rescan-report.json"
lvm_cache_rescan_receipt="$tmpdir/lvm-cache-rescan-receipt.json"

jq -n '{
  lvmCaches: {
    "vg0/archive": {
      operation: "rescan"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$lvm_cache_rescan_spec"

if PATH="$lvm_cache_rescan_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$lvm_cache_rescan_spec" \
  --execute \
  --report-out "$lvm_cache_rescan_report" \
  --receipt-out "$lvm_cache_rescan_receipt" \
  --json > "$lvm_cache_rescan_json"; then
  echo "expected synthetic LVM cache rescan failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 1
  and .executionResults[0].success == false
  and .executionResults[0].statusCode == 92
  and .executionResults[0].argv == ["lvs", "--reportformat", "json", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/archive"]
  and (.executionResults[0].stderr | contains("synthetic lvm cache rescan failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "lvmcaches:vg0/archive:rescan"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["lvs", "--reportformat", "json", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/archive"]
  and .partialExecutionRecovery.retryReviewActionIds == ["lvmcaches:vg0/archive:rescan"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/archive"]))
    and (.commands | any(.argv == ["vgs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["pvs", "--reportformat", "json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "vg0/archive", "--json"]))
    and (.notes | any(contains("cache changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/archive"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["lvs", "--reportformat", "json", "-a", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/archive"]))
  ))
' "$lvm_cache_rescan_json" >/dev/null

cmp "$lvm_cache_rescan_json" "$lvm_cache_rescan_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "lvmcaches:vg0/archive:rescan"
  and .report.partialExecutionRecovery.failedCommand == ["lvs", "--reportformat", "json", "-o", "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent", "vg0/archive"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$lvm_cache_rescan_receipt" >/dev/null

vdo_create_tools="$tmpdir/fake-vdo-create-tools"
mkdir -p "$vdo_create_tools"
vdo_create_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$vdo_create_tools/vdo" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "create" ]]; then
  echo "synthetic VDO create failure for disk-nix recovery coverage" >&2
  exit 90
fi
printf '{}\n'
EOF

cat > "$vdo_create_tools/vdostats" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$vdo_create_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$vdo_create_disk_nix" "\$@"
EOF

chmod +x "$vdo_create_tools/vdo" "$vdo_create_tools/vdostats" "$vdo_create_tools/disk-nix"

vdo_create_spec="$tmpdir/vdo-create-spec.json"
vdo_create_json="$tmpdir/vdo-create-apply.json"
vdo_create_report="$tmpdir/vdo-create-report.json"
vdo_create_receipt="$tmpdir/vdo-create-receipt.json"

jq -n '{
  vdoVolumes: {
    "new-cache": {
      operation: "create",
      device: "/dev/disk/by-id/vdo-backing",
      desiredSize: "2TiB"
    }
  },
  apply: {
    allowDestructive: true
  }
}' > "$vdo_create_spec"

if PATH="$vdo_create_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$vdo_create_spec" \
  --execute \
  --report-out "$vdo_create_report" \
  --receipt-out "$vdo_create_receipt" \
  --json > "$vdo_create_json"; then
  echo "expected synthetic VDO create failure to fail apply" >&2
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
  and .executionResults[0].actionId == "vdovolumes:new-cache:create"
  and .executionResults[0].argv == ["disk-nix", "inspect", "/dev/disk/by-id/vdo-backing"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 90
  and .executionResults[1].actionId == "vdovolumes:new-cache:create"
  and .executionResults[1].argv == ["vdo", "create", "--name", "new-cache", "--device", "/dev/disk/by-id/vdo-backing", "--vdoLogicalSize", "2TiB"]
  and (.executionResults[1].stderr | contains("synthetic VDO create failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "vdovolumes:new-cache:create"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["vdo", "create", "--name", "new-cache", "--device", "/dev/disk/by-id/vdo-backing", "--vdoLogicalSize", "2TiB"]
  and .partialExecutionRecovery.retryReviewActionIds == ["vdovolumes:new-cache:create"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["vdo", "status", "--name", "new-cache"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "new-cache"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
    and (.notes | any(contains("VDO lifecycle changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["disk-nix", "inspect", "new-cache", "--json"]))
    and (.commands | any(.argv == ["vdo", "status", "--name", "new-cache"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "new-cache"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["vdo", "status", "--name", "new-cache"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "new-cache"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$vdo_create_json" >/dev/null

cmp "$vdo_create_json" "$vdo_create_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "vdovolumes:new-cache:create"
  and .report.partialExecutionRecovery.failedCommand == ["vdo", "create", "--name", "new-cache", "--device", "/dev/disk/by-id/vdo-backing", "--vdoLogicalSize", "2TiB"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$vdo_create_receipt" >/dev/null

vdo_rescan_tools="$tmpdir/fake-vdo-rescan-tools"
mkdir -p "$vdo_rescan_tools"

cat > "$vdo_rescan_tools/vdo" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$vdo_rescan_tools/vdostats" <<'EOF'
#!/usr/bin/env bash
echo "synthetic VDO rescan stats failure for disk-nix recovery coverage" >&2
exit 91
EOF

chmod +x "$vdo_rescan_tools/vdo" "$vdo_rescan_tools/vdostats"

vdo_rescan_spec="$tmpdir/vdo-rescan-spec.json"
vdo_rescan_json="$tmpdir/vdo-rescan-apply.json"
vdo_rescan_report="$tmpdir/vdo-rescan-report.json"
vdo_rescan_receipt="$tmpdir/vdo-rescan-receipt.json"

jq -n '{
  vdoVolumes: {
    refreshArchive: {
      operation: "rescan"
    }
  }
}' > "$vdo_rescan_spec"

if PATH="$vdo_rescan_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$vdo_rescan_spec" \
  --execute \
  --report-out "$vdo_rescan_report" \
  --receipt-out "$vdo_rescan_receipt" \
  --json > "$vdo_rescan_json"; then
  echo "expected synthetic VDO rescan failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 1
  and .commandSummary.commandCount == 3
  and .commandSummary.mutatingCount == 0
  and .commandSummary.manualReviewCount == 1
  and .commandSummary.readyCount == 3
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "vdovolumes:refresharchive:rescan"
  and .executionResults[0].argv == ["vdo", "status", "--name", "refreshArchive"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 91
  and .executionResults[1].actionId == "vdovolumes:refresharchive:rescan"
  and .executionResults[1].argv == ["vdostats", "--human-readable", "refreshArchive"]
  and (.executionResults[1].stderr | contains("synthetic VDO rescan stats failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "vdovolumes:refresharchive:rescan"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["vdostats", "--human-readable", "refreshArchive"]
  and .partialExecutionRecovery.retryReviewActionIds == ["vdovolumes:refresharchive:rescan"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(.kind == "review-execution-failure"))
  and (.recoveryActions | any(
    .kind == "inspect-current-state"
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "resume-after-fix"))
  and (.recoveryActions | all(.kind != "domain-recovery"))
' "$vdo_rescan_json" >/dev/null

cmp "$vdo_rescan_json" "$vdo_rescan_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "vdovolumes:refresharchive:rescan"
  and .report.partialExecutionRecovery.failedCommand == ["vdostats", "--human-readable", "refreshArchive"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$vdo_rescan_receipt" >/dev/null

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

vdo_physical_grow_tools="$tmpdir/fake-vdo-physical-grow-tools"
mkdir -p "$vdo_physical_grow_tools"
vdo_physical_grow_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$vdo_physical_grow_tools/vdo" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "growPhysical" ]]; then
  echo "synthetic VDO physical grow failure for disk-nix recovery coverage" >&2
  exit 92
fi
printf '{}\n'
EOF

cat > "$vdo_physical_grow_tools/vdostats" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$vdo_physical_grow_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$vdo_physical_grow_disk_nix" "\$@"
EOF

chmod +x "$vdo_physical_grow_tools/vdo" "$vdo_physical_grow_tools/vdostats" "$vdo_physical_grow_tools/disk-nix"

vdo_physical_grow_spec="$tmpdir/vdo-physical-grow-spec.json"
vdo_physical_grow_json="$tmpdir/vdo-physical-grow-apply.json"
vdo_physical_grow_report="$tmpdir/vdo-physical-grow-report.json"
vdo_physical_grow_receipt="$tmpdir/vdo-physical-grow-receipt.json"

jq -n '{
  vdoVolumes: {
    "archive-physical": {
      operation: "grow",
      physicalSize: "6TiB"
    }
  },
  apply: {
    allowGrow: true
  }
}' > "$vdo_physical_grow_spec"

if PATH="$vdo_physical_grow_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$vdo_physical_grow_spec" \
  --execute \
  --report-out "$vdo_physical_grow_report" \
  --receipt-out "$vdo_physical_grow_receipt" \
  --json > "$vdo_physical_grow_json"; then
  echo "expected synthetic VDO physical grow failure to fail apply" >&2
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
  and .executionResults[0].actionId == "vdovolumes:archive-physical:grow"
  and .executionResults[0].argv == ["vdo", "status", "--name", "archive-physical"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 92
  and .executionResults[1].actionId == "vdovolumes:archive-physical:grow"
  and .executionResults[1].argv == ["vdo", "growPhysical", "--name", "archive-physical"]
  and (.executionResults[1].stderr | contains("synthetic VDO physical grow failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "vdovolumes:archive-physical:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["vdo", "growPhysical", "--name", "archive-physical"]
  and .partialExecutionRecovery.retryReviewActionIds == ["vdovolumes:archive-physical:grow"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["vdo", "status", "--name", "archive-physical"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "archive-physical"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
    and (.notes | any(contains("VDO lifecycle changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["disk-nix", "inspect", "archive-physical", "--json"]))
    and (.commands | any(.argv == ["vdo", "status", "--name", "archive-physical"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "archive-physical"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["vdo", "status", "--name", "archive-physical"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "archive-physical"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$vdo_physical_grow_json" >/dev/null

cmp "$vdo_physical_grow_json" "$vdo_physical_grow_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "vdovolumes:archive-physical:grow"
  and .report.partialExecutionRecovery.failedCommand == ["vdo", "growPhysical", "--name", "archive-physical"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$vdo_physical_grow_receipt" >/dev/null

vdo_start_tools="$tmpdir/fake-vdo-start-tools"
mkdir -p "$vdo_start_tools"
vdo_start_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$vdo_start_tools/vdo" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "start --name warmArchive" ]]; then
  echo "synthetic VDO start failure for disk-nix recovery coverage" >&2
  exit 87
fi
printf '{}\n'
EOF

cat > "$vdo_start_tools/vdostats" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$vdo_start_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$vdo_start_disk_nix" "\$@"
EOF

chmod +x "$vdo_start_tools/vdo" "$vdo_start_tools/vdostats" "$vdo_start_tools/disk-nix"

vdo_start_spec="$tmpdir/vdo-start-spec.json"
vdo_start_json="$tmpdir/vdo-start-apply.json"
vdo_start_report="$tmpdir/vdo-start-report.json"
vdo_start_receipt="$tmpdir/vdo-start-receipt.json"

jq -n '{
  vdoVolumes: {
    warmArchive: {
      operation: "start"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$vdo_start_spec"

if PATH="$vdo_start_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$vdo_start_spec" \
  --execute \
  --report-out "$vdo_start_report" \
  --receipt-out "$vdo_start_receipt" \
  --json > "$vdo_start_json"; then
  echo "expected synthetic VDO start failure to fail apply" >&2
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
  and .executionResults[0].actionId == "vdovolumes:warmarchive:start"
  and .executionResults[0].argv == ["vdo", "status", "--name", "warmArchive"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 87
  and .executionResults[1].actionId == "vdovolumes:warmarchive:start"
  and .executionResults[1].argv == ["vdo", "start", "--name", "warmArchive"]
  and (.executionResults[1].stderr | contains("synthetic VDO start failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "vdovolumes:warmarchive:start"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["vdo", "start", "--name", "warmArchive"]
  and .partialExecutionRecovery.retryReviewActionIds == ["vdovolumes:warmarchive:start"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["vdo", "status", "--name", "warmarchive"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "warmarchive"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
    and (.notes | any(contains("VDO lifecycle changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["disk-nix", "inspect", "warmarchive", "--json"]))
    and (.commands | any(.argv == ["vdo", "status", "--name", "warmarchive"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "warmarchive"]))
    and (.commands | any(.argv == ["vdo", "status", "--name", "warmArchive"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "warmArchive"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "warmArchive", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["vdo", "status", "--name", "warmarchive"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "warmarchive"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$vdo_start_json" >/dev/null

cmp "$vdo_start_json" "$vdo_start_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "vdovolumes:warmarchive:start"
  and .report.partialExecutionRecovery.failedCommand == ["vdo", "start", "--name", "warmArchive"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$vdo_start_receipt" >/dev/null

vdo_stop_tools="$tmpdir/fake-vdo-stop-tools"
mkdir -p "$vdo_stop_tools"
vdo_stop_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$vdo_stop_tools/vdo" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "stop --name coldArchive" ]]; then
  echo "synthetic VDO stop failure for disk-nix recovery coverage" >&2
  exit 88
fi
printf '{}\n'
EOF

cat > "$vdo_stop_tools/vdostats" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$vdo_stop_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$vdo_stop_disk_nix" "\$@"
EOF

chmod +x "$vdo_stop_tools/vdo" "$vdo_stop_tools/vdostats" "$vdo_stop_tools/disk-nix"

vdo_stop_spec="$tmpdir/vdo-stop-spec.json"
vdo_stop_json="$tmpdir/vdo-stop-apply.json"
vdo_stop_report="$tmpdir/vdo-stop-report.json"
vdo_stop_receipt="$tmpdir/vdo-stop-receipt.json"

jq -n '{
  vdoVolumes: {
    coldArchive: {
      operation: "stop"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$vdo_stop_spec"

if PATH="$vdo_stop_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$vdo_stop_spec" \
  --execute \
  --report-out "$vdo_stop_report" \
  --receipt-out "$vdo_stop_receipt" \
  --json > "$vdo_stop_json"; then
  echo "expected synthetic VDO stop failure to fail apply" >&2
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
  and .executionResults[0].actionId == "vdovolumes:coldarchive:stop"
  and .executionResults[0].argv == ["vdo", "status", "--name", "coldArchive"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 88
  and .executionResults[1].actionId == "vdovolumes:coldarchive:stop"
  and .executionResults[1].argv == ["vdo", "stop", "--name", "coldArchive"]
  and (.executionResults[1].stderr | contains("synthetic VDO stop failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "vdovolumes:coldarchive:stop"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["vdo", "stop", "--name", "coldArchive"]
  and .partialExecutionRecovery.retryReviewActionIds == ["vdovolumes:coldarchive:stop"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["vdo", "status", "--name", "coldarchive"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "coldarchive"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
    and (.notes | any(contains("VDO lifecycle changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["disk-nix", "inspect", "coldarchive", "--json"]))
    and (.commands | any(.argv == ["vdo", "status", "--name", "coldarchive"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "coldarchive"]))
    and (.commands | any(.argv == ["vdo", "status"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["vdo", "status", "--name", "coldarchive"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "coldarchive"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$vdo_stop_json" >/dev/null

cmp "$vdo_stop_json" "$vdo_stop_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "vdovolumes:coldarchive:stop"
  and .report.partialExecutionRecovery.failedCommand == ["vdo", "stop", "--name", "coldArchive"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$vdo_stop_receipt" >/dev/null
