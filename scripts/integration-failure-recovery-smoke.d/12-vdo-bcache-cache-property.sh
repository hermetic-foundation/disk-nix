vdo_remove_tools="$tmpdir/fake-vdo-remove-tools"
mkdir -p "$vdo_remove_tools"
vdo_remove_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$vdo_remove_tools/vdo" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "remove --name old-cache" ]]; then
  echo "synthetic VDO remove failure for disk-nix recovery coverage" >&2
  exit 89
fi
printf '{}\n'
EOF

cat > "$vdo_remove_tools/vdostats" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$vdo_remove_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$vdo_remove_disk_nix" "\$@"
EOF

chmod +x "$vdo_remove_tools/vdo" "$vdo_remove_tools/vdostats" "$vdo_remove_tools/disk-nix"

vdo_remove_spec="$tmpdir/vdo-remove-spec.json"
vdo_remove_json="$tmpdir/vdo-remove-apply.json"
vdo_remove_report="$tmpdir/vdo-remove-report.json"
vdo_remove_receipt="$tmpdir/vdo-remove-receipt.json"

jq -n '{
  vdoVolumes: {
    "old-cache": {
      destroy: true
    }
  },
  apply: {
    allowDestructive: true
  }
}' > "$vdo_remove_spec"

if PATH="$vdo_remove_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$vdo_remove_spec" \
  --execute \
  --report-out "$vdo_remove_report" \
  --receipt-out "$vdo_remove_receipt" \
  --json > "$vdo_remove_json"; then
  echo "expected synthetic VDO remove failure to fail apply" >&2
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
  and .executionResults[0].actionId == "vdovolumes:old-cache:destroy"
  and .executionResults[0].argv == ["vdo", "status", "--name", "old-cache"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 89
  and .executionResults[1].actionId == "vdovolumes:old-cache:destroy"
  and .executionResults[1].argv == ["vdo", "remove", "--name", "old-cache"]
  and (.executionResults[1].stderr | contains("synthetic VDO remove failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "vdovolumes:old-cache:destroy"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["vdo", "remove", "--name", "old-cache"]
  and .partialExecutionRecovery.retryReviewActionIds == ["vdovolumes:old-cache:destroy"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["vdo", "status", "--name", "old-cache"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "old-cache"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
    and (.notes | any(contains("VDO lifecycle changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["disk-nix", "inspect", "old-cache", "--json"]))
    and (.commands | any(.argv == ["vdo", "status", "--name", "old-cache"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "old-cache"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
    and (.commands | any(.argv == ["vdo", "status"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["vdo", "status", "--name", "old-cache"]))
    and (.commands | any(.argv == ["vdostats", "--human-readable", "old-cache"]))
    and (.commands | any(.argv == ["disk-nix", "vdo", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$vdo_remove_json" >/dev/null

cmp "$vdo_remove_json" "$vdo_remove_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "vdovolumes:old-cache:destroy"
  and .report.partialExecutionRecovery.failedCommand == ["vdo", "remove", "--name", "old-cache"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$vdo_remove_receipt" >/dev/null

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

bcache_replace_tools="$tmpdir/fake-bcache-replace-tools"
mkdir -p "$bcache_replace_tools"
bcache_replace_disk_nix="$(command -v "$disk_nix_bin")"
bcache_replace_real_sh="$(command -v sh)"

cat > "$bcache_replace_tools/sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" == "$bcache_replace_real_sh" || "\${1:-}" == "/bin/sh" ]]; then
  shift
fi
case "\$*" in
*"command -v"*)
  exit 0
  ;;
*"disk-nix-bcache-replace /dev/bcache0 /dev/disk/by-id/new-cache 11111111-2222-3333-4444-555555555555"*)
  echo "synthetic bcache replacement failure for disk-nix recovery coverage" >&2
  exit 87
  ;;
esac
exec "$bcache_replace_real_sh" "\$@"
EOF

cat > "$bcache_replace_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$bcache_replace_disk_nix" "\$@"
EOF

chmod +x "$bcache_replace_tools/sh" "$bcache_replace_tools/disk-nix"

bcache_replace_spec="$tmpdir/bcache-replace-spec.json"
bcache_replace_json="$tmpdir/bcache-replace-apply.json"
bcache_replace_report="$tmpdir/bcache-replace-report.json"
bcache_replace_receipt="$tmpdir/bcache-replace-receipt.json"

jq -n '{
  caches: {
    "/dev/bcache0": {
      replaceDevices: {
        "/dev/disk/by-id/old-cache": "/dev/disk/by-id/new-cache"
      },
      cacheSetUuid: "11111111-2222-3333-4444-555555555555"
    }
  },
  apply: {
    allowOffline: true,
    allowDeviceReplacement: true
  }
}' > "$bcache_replace_spec"

if PATH="$bcache_replace_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$bcache_replace_spec" \
  --execute \
  --report-out "$bcache_replace_report" \
  --receipt-out "$bcache_replace_receipt" \
  --json > "$bcache_replace_json"; then
  echo "expected synthetic bcache replacement failure to fail apply" >&2
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
  and .executionResults[0].actionId == "caches:/dev/bcache0:replace-device:/dev/disk/by-id/old-cache"
  and .executionResults[0].argv == ["disk-nix", "inspect", "/dev/bcache0"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 87
  and .executionResults[1].actionId == "caches:/dev/bcache0:replace-device:/dev/disk/by-id/old-cache"
  and .executionResults[1].argv == ["sh", "-c", "make-bcache -C \"$2\" --cset-uuid \"$3\" --writeback && printf '\''1\\n'\'' > \"/sys/block/${1#/dev/}/bcache/detach\" && printf '\''%s\\n'\'' \"$3\" > \"/sys/block/${1#/dev/}/bcache/attach\"", "disk-nix-bcache-replace", "/dev/bcache0", "/dev/disk/by-id/new-cache", "11111111-2222-3333-4444-555555555555"]
  and (.executionResults[1].stderr | contains("synthetic bcache replacement failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "caches:/dev/bcache0:replace-device:/dev/disk/by-id/old-cache"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["sh", "-c", "make-bcache -C \"$2\" --cset-uuid \"$3\" --writeback && printf '\''1\\n'\'' > \"/sys/block/${1#/dev/}/bcache/detach\" && printf '\''%s\\n'\'' \"$3\" > \"/sys/block/${1#/dev/}/bcache/attach\"", "disk-nix-bcache-replace", "/dev/bcache0", "/dev/disk/by-id/new-cache", "11111111-2222-3333-4444-555555555555"]
  and .partialExecutionRecovery.retryReviewActionIds == ["caches:/dev/bcache0:replace-device:/dev/disk/by-id/old-cache"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache0", "state"]))
    and (.commands | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache0", "cache_mode"]))
    and (.commands | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache0", "dirty_data"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/bcache0", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "cache", "--json"]))
    and (.notes | any(contains("cache changes")))
    and (.notes | any(contains("dirty-data")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/bcache0", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "cache", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache0", "state"]))
    and (.commands | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache0", "dirty_data"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$bcache_replace_json" >/dev/null

cmp "$bcache_replace_json" "$bcache_replace_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "caches:/dev/bcache0:replace-device:/dev/disk/by-id/old-cache"
  and .report.partialExecutionRecovery.failedCommand == ["sh", "-c", "make-bcache -C \"$2\" --cset-uuid \"$3\" --writeback && printf '\''1\\n'\'' > \"/sys/block/${1#/dev/}/bcache/detach\" && printf '\''%s\\n'\'' \"$3\" > \"/sys/block/${1#/dev/}/bcache/attach\"", "disk-nix-bcache-replace", "/dev/bcache0", "/dev/disk/by-id/new-cache", "11111111-2222-3333-4444-555555555555"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$bcache_replace_receipt" >/dev/null

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

bcache_rescan_tools="$tmpdir/fake-bcache-rescan-tools"
mkdir -p "$bcache_rescan_tools"
bcache_rescan_disk_nix="$(command -v "$disk_nix_bin")"
bcache_rescan_real_sh="$(command -v sh)"

cat > "$bcache_rescan_tools/sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" == "$bcache_rescan_real_sh" || "\${1:-}" == "/bin/sh" ]]; then
  shift
fi
case "\$*" in
*"command -v"*)
  exit 0
  ;;
*"disk-nix-bcache-read /dev/bcache0 state"*)
  echo "synthetic bcache rescan failure for disk-nix recovery coverage" >&2
  exit 93
  ;;
esac
exec "$bcache_rescan_real_sh" "\$@"
EOF

cat > "$bcache_rescan_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$bcache_rescan_disk_nix" "\$@"
EOF

chmod +x "$bcache_rescan_tools/sh" "$bcache_rescan_tools/disk-nix"

bcache_rescan_spec="$tmpdir/bcache-rescan-spec.json"
bcache_rescan_json="$tmpdir/bcache-rescan-apply.json"
bcache_rescan_report="$tmpdir/bcache-rescan-report.json"
bcache_rescan_receipt="$tmpdir/bcache-rescan-receipt.json"

jq -n '{
  caches: {
    "/dev/bcache0": {
      operation: "rescan"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$bcache_rescan_spec"

if PATH="$bcache_rescan_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$bcache_rescan_spec" \
  --execute \
  --report-out "$bcache_rescan_report" \
  --receipt-out "$bcache_rescan_receipt" \
  --json > "$bcache_rescan_json"; then
  echo "expected synthetic bcache rescan failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 4
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["disk-nix", "inspect", "/dev/bcache0"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 93
  and .executionResults[1].argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache0", "state"]
  and (.executionResults[1].stderr | contains("synthetic bcache rescan failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "caches:/dev/bcache0:rescan"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache0", "state"]
  and .partialExecutionRecovery.retryReviewActionIds == ["caches:/dev/bcache0:rescan"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache0", "state"]))
    and (.commands | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache0", "cache_mode"]))
    and (.commands | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache0", "dirty_data"]))
    and (.commands | any(.argv == ["disk-nix", "cache", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/bcache0", "--json"]))
    and (.notes | any(contains("cache changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache0", "cache_mode"]))
  ))
' "$bcache_rescan_json" >/dev/null

cmp "$bcache_rescan_json" "$bcache_rescan_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "caches:/dev/bcache0:rescan"
  and .report.partialExecutionRecovery.failedCommand == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", "/dev/bcache0", "state"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$bcache_rescan_receipt" >/dev/null

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

echo "failure-recovery integration smoke test verified partialExecutionRecovery after synthetic resize, LVM grow, LVM thin-pool create, LVM thin-pool grow, XFS grow, Btrfs scrub, Btrfs rebalance, Btrfs device replacement, bcachefs replacement, filesystem trim, filesystem check, filesystem repair, filesystem property, swap label, zram rescan, zram property inventory, loop rescan, backing-file rescan, backing-file grow, backing-file create, device-mapper rename, ZFS dataset rename, Btrfs snapshot clone, ZFS snapshot clone, LVM VG rename, LVM VG replacement, ZFS pool replacement, ZFS rollback, NVMe namespace create, NVMe namespace grow, NVMe namespace attach, NVMe namespace detach, NVMe namespace delete, target-side LUN LIO create, target-side LUN LIO attach, target-side LUN LIO detach, target-side LUN LIO destroy, target-side LUN LIO native grow with backing capacity and host verification, target-side LUN LIO property, target-side LUN LIO rescan, target-side LUN tgt create, target-side LUN tgt attach, target-side LUN tgt detach, target-side LUN tgt destroy, target-side LUN tgt native grow with backing capacity and host verification, target-side LUN tgt property, target-side LUN tgt rescan, target-side LUN SCST create, target-side LUN SCST attach, target-side LUN SCST detach, target-side LUN SCST destroy, target-side LUN SCST grow, target-side LUN SCST property, target-side LUN SCST rescan, host-side LUN rescan, multipath add, multipath remove, multipath flush, multipath resize, multipath replace, MD RAID create, MD RAID assemble, MD RAID stop, MD RAID grow, MD RAID add-member, MD RAID remove-member, MD RAID replace, LUKS open, LUKS format, LUKS close, LUKS grow, LUKS keyslot add, LUKS token import, LUKS keyslot remove, LUKS token remove, LUKS property, partition grow, NFS remount, NFS unmount, NFS export, NFS unexport, iSCSI logout, iSCSI login, iSCSI rescan, LVM cache attach, LVM cache detach, LVM cache replacement, LVM cache rescan, VDO create, VDO rescan, VDO logical grow, VDO physical grow, VDO start, VDO stop, VDO remove, VDO property, bcache replacement, bcache property, bcache rescan, and LVM cache property failures"
