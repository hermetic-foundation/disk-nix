target_lun_scst_tools="$tmpdir/fake-target-lun-scst-tools"
mkdir -p "$target_lun_scst_tools"

cat > "$target_lun_scst_tools/scstadmin" <<'EOF'
#!/usr/bin/env bash
case "$*" in
*"-add_lun 9"*)
  echo "synthetic SCST target-side LUN add_lun failure for disk-nix recovery coverage" >&2
  exit 96
  ;;
*)
  printf 'scst ok\n'
  ;;
esac
EOF

chmod +x "$target_lun_scst_tools/scstadmin"

target_lun_scst_spec="$tmpdir/target-lun-scst-spec.json"
target_lun_scst_json="$tmpdir/target-lun-scst-apply.json"
target_lun_scst_report="$tmpdir/target-lun-scst-report.json"
target_lun_scst_receipt="$tmpdir/target-lun-scst-receipt.json"

jq -n '{
  targetLuns: {
    "iqn.2026-06.example:scst.root": {
      operation: "create",
      provider: "scst",
      source: "/dev/zvol/tank/root",
      lun: 9,
      group: "hosts",
      client: "iqn.2026-06.example:host.primary"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$target_lun_scst_spec"

if PATH="$target_lun_scst_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_scst_spec" \
  --execute \
  --report-out "$target_lun_scst_report" \
  --receipt-out "$target_lun_scst_receipt" \
  --json > "$target_lun_scst_json"; then
  echo "expected synthetic target-side LUN SCST create failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 1
  and .commandSummary.commandCount == 9
  and .commandSummary.mutatingCount == 7
  and .commandSummary.manualReviewCount == 1
  and .commandSummary.readyCount == 9
  and .commandSummary.needsDomainImplementationCount == 0
  and (.executionResults | length) == 6
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["scstadmin", "-list_target", "iqn.2026-06.example:scst.root", "-driver", "iscsi"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["scstadmin", "-open_dev", "_dev_zvol_tank_root", "-handler", "vdisk_blockio", "-attributes", "filename=/dev/zvol/tank/root"]
  and .executionResults[2].success == true
  and .executionResults[2].argv == ["scstadmin", "-add_target", "iqn.2026-06.example:scst.root", "-driver", "iscsi"]
  and .executionResults[3].success == true
  and .executionResults[3].argv == ["scstadmin", "-add_group", "hosts", "-driver", "iscsi", "-target", "iqn.2026-06.example:scst.root"]
  and .executionResults[4].success == true
  and .executionResults[4].argv == ["scstadmin", "-add_init", "iqn.2026-06.example:host.primary", "-driver", "iscsi", "-target", "iqn.2026-06.example:scst.root", "-group", "hosts"]
  and .executionResults[5].success == false
  and .executionResults[5].statusCode == 96
  and .executionResults[5].argv == ["scstadmin", "-add_lun", "9", "-driver", "iscsi", "-target", "iqn.2026-06.example:scst.root", "-group", "hosts", "-device", "_dev_zvol_tank_root"]
  and (.executionResults[5].stderr | contains("synthetic SCST target-side LUN add_lun failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:scst.root:create"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["scstadmin", "-add_lun", "9", "-driver", "iscsi", "-target", "iqn.2026-06.example:scst.root", "-group", "hosts", "-device", "_dev_zvol_tank_root"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetluns:iqn.2026-06.example:scst.root:create"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 4
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(.kind == "review-execution-failure"))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["targetcli", "/iscsi", "ls"]))
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:scst.root", "ls"]))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
    and (.commands | any(.argv == ["multipath", "-ll"]))
    and (.notes | any(contains("target-side LUN changes")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["scstadmin", "-list_target", "iqn.2026-06.example:scst.root", "-driver", "iscsi"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(.kind == "resume-after-fix"))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$target_lun_scst_json" >/dev/null

cmp "$target_lun_scst_json" "$target_lun_scst_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:scst.root:create"
  and .report.partialExecutionRecovery.failedCommand == ["scstadmin", "-add_lun", "9", "-driver", "iscsi", "-target", "iqn.2026-06.example:scst.root", "-group", "hosts", "-device", "_dev_zvol_tank_root"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 4
' "$target_lun_scst_receipt" >/dev/null

run_scst_failure_case() {
  local name="$1"
  local fail_match="$2"
  local spec_json="$3"
  local failed_action="$4"
  local failed_command_json="$5"
  local completed_mutating="$6"
  local command_count="$7"
  local result_count="$8"
  local status_code="$9"

  local tools="$tmpdir/fake-target-lun-scst-$name-tools"
  mkdir -p "$tools"

  cat > "$tools/scstadmin" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "${DISK_NIX_SCST_FAIL_MATCH:-}" ]]; then
  echo "synthetic SCST target-side LUN ${DISK_NIX_SCST_CASE:-operation} failure for disk-nix recovery coverage" >&2
  exit "${DISK_NIX_SCST_STATUS:-97}"
fi
printf 'scst ok\n'
EOF

  chmod +x "$tools/scstadmin"

  local spec="$tmpdir/target-lun-scst-$name-spec.json"
  local json="$tmpdir/target-lun-scst-$name-apply.json"
  local report="$tmpdir/target-lun-scst-$name-report.json"
  local receipt="$tmpdir/target-lun-scst-$name-receipt.json"

  jq -n "$spec_json" > "$spec"

  if DISK_NIX_SCST_FAIL_MATCH="$fail_match" \
    DISK_NIX_SCST_CASE="$name" \
    DISK_NIX_SCST_STATUS="$status_code" \
    PATH="$tools:$PATH" "$disk_nix_bin" apply \
      --spec "$spec" \
      --execute \
      --report-out "$report" \
      --receipt-out "$receipt" \
      --json > "$json"; then
    echo "expected synthetic target-side LUN SCST $name failure to fail apply" >&2
    exit 1
  fi

  jq -e \
    --arg action "$failed_action" \
    --arg name "$name" \
    --argjson failed "$failed_command_json" \
    --argjson completed "$completed_mutating" \
    --argjson commands "$command_count" \
    --argjson results "$result_count" \
    --argjson code "$status_code" '
    .status == "failed"
    and .apply.blockedCount == 0
    and .commandSummary.stepCount == 1
    and .commandSummary.commandCount == $commands
    and .commandSummary.needsDomainImplementationCount == 0
    and (.executionResults | length) == $results
    and .executionResults[-1].success == false
    and .executionResults[-1].statusCode == $code
    and .executionResults[-1].argv == $failed
    and (.executionResults[-1].stderr | contains("synthetic SCST target-side LUN " + $name + " failure"))
    and .partialExecutionRecovery.completedActionIds == []
    and .partialExecutionRecovery.failedActionId == $action
    and .partialExecutionRecovery.failedPhase == "command"
    and .partialExecutionRecovery.failedCommand == $failed
    and .partialExecutionRecovery.retryReviewActionIds == [$action]
    and .partialExecutionRecovery.remainingActionIds == []
    and .partialExecutionRecovery.completedMutatingCommandCount == $completed
    and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
    and (.recoveryActions | any(.kind == "review-execution-failure"))
    and (.recoveryActions | any(
      .kind == "domain-recovery"
      and (.commands | any(.argv == ["targetcli", "/iscsi", "ls"]))
      and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:scst.root", "ls"]))
      and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
      and (.commands | any(.argv == ["lsscsi", "-t", "-s"]))
      and (.commands | any(.argv == ["multipath", "-ll"]))
      and (.notes | any(contains("target-side LUN changes")))
    ))
    and (.recoveryActions | any(
      .kind == "roll-forward-review"
      and (.commands | any(.argv == ["scstadmin", "-list_target", "iqn.2026-06.example:scst.root", "-driver", "iscsi"]))
      and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    ))
    and (.recoveryActions | any(.kind == "rollback-review" and (.commands | all(.mutates == false))))
    and (.recoveryActions | any(.kind == "preserve-recovery-points"))
  ' "$json" >/dev/null

  cmp "$json" "$report" >/dev/null
  jq -e \
    --arg action "$failed_action" \
    --argjson failed "$failed_command_json" \
    --argjson completed "$completed_mutating" '
    .receiptVersion == 1
    and .command == "apply"
    and .executeRequested == true
    and .report.status == "failed"
    and .report.partialExecutionRecovery.failedActionId == $action
    and .report.partialExecutionRecovery.failedCommand == $failed
    and .report.partialExecutionRecovery.completedMutatingCommandCount == $completed
  ' "$receipt" >/dev/null
}

scst_common_apply='
  apply: {
    allowOffline: true,
    allowGrow: true,
    allowDestructive: true,
    allowPotentialDataLoss: true,
    allowPropertyChanges: true,
    backupVerified: true
  }'

run_scst_failure_case \
  "attach" \
  "-add_lun 9 -driver iscsi -target iqn.2026-06.example:scst.root -group hosts -device _dev_zvol_tank_root" \
  "{
    targetLuns: {
      \"iqn.2026-06.example:scst.root\": {
        operation: \"attach\",
        provider: \"scst\",
        source: \"/dev/zvol/tank/root\",
        lun: 9,
        group: \"hosts\",
        client: \"iqn.2026-06.example:host.primary\"
      }
    },
    $scst_common_apply
  }" \
  "targetluns:iqn.2026-06.example:scst.root:attach" \
  '["scstadmin", "-add_lun", "9", "-driver", "iscsi", "-target", "iqn.2026-06.example:scst.root", "-group", "hosts", "-device", "_dev_zvol_tank_root"]' \
  1 7 3 97

run_scst_failure_case \
  "detach" \
  "-rem_lun 9 -driver iscsi -target iqn.2026-06.example:scst.root -group hosts" \
  "{
    targetLuns: {
      \"iqn.2026-06.example:scst.root\": {
        operation: \"detach\",
        provider: \"scst\",
        source: \"/dev/zvol/tank/root\",
        lun: 9,
        group: \"hosts\",
        client: \"iqn.2026-06.example:host.primary\"
      }
    },
    $scst_common_apply
  }" \
  "targetluns:iqn.2026-06.example:scst.root:detach" \
  '["scstadmin", "-rem_lun", "9", "-driver", "iscsi", "-target", "iqn.2026-06.example:scst.root", "-group", "hosts"]' \
  2 6 4 98

run_scst_failure_case \
  "destroy" \
  "-rem_target iqn.2026-06.example:scst.root -driver iscsi" \
  "{
    targetLuns: {
      \"iqn.2026-06.example:scst.root\": {
        destroy: true,
        provider: \"scst\",
        source: \"/dev/zvol/tank/root\",
        lun: 9,
        group: \"hosts\",
        client: \"iqn.2026-06.example:host.primary\"
      }
    },
    $scst_common_apply
  }" \
  "targetluns:iqn.2026-06.example:scst.root:destroy" \
  '["scstadmin", "-rem_target", "iqn.2026-06.example:scst.root", "-driver", "iscsi"]' \
  3 8 5 99

run_scst_failure_case \
  "grow" \
  "-resync_dev _dev_zvol_tank_root" \
  "{
    targetLuns: {
      \"iqn.2026-06.example:scst.root\": {
        operation: \"grow\",
        provider: \"scst\",
        source: \"/dev/zvol/tank/root\",
        desiredSize: \"4TiB\",
        lun: 9,
        group: \"hosts\"
      }
    },
    $scst_common_apply
  }" \
  "targetluns:iqn.2026-06.example:scst.root:grow" \
  '["scstadmin", "-resync_dev", "_dev_zvol_tank_root"]' \
  0 4 3 100

run_scst_failure_case \
  "property" \
  "-set_lun_attr 9 -driver iscsi -target iqn.2026-06.example:scst.root -group hosts -attributes read_only=0" \
  "{
    targetLuns: {
      \"iqn.2026-06.example:scst.root\": {
        provider: \"scst\",
        source: \"/dev/zvol/tank/root\",
        lun: 9,
        group: \"hosts\",
        properties: {
          read_only: \"0\"
        }
      }
    },
    $scst_common_apply
  }" \
  "targetLuns:iqn.2026-06.example:scst.root:set-property:read_only" \
  '["scstadmin", "-set_lun_attr", "9", "-driver", "iscsi", "-target", "iqn.2026-06.example:scst.root", "-group", "hosts", "-attributes", "read_only=0"]' \
  0 4 2 101

run_scst_failure_case \
  "rescan" \
  "-resync_dev _dev_zvol_tank_root" \
  "{
    targetLuns: {
      \"iqn.2026-06.example:scst.root\": {
        operation: \"rescan\",
        provider: \"scst\",
        source: \"/dev/zvol/tank/root\",
        lun: 9,
        group: \"hosts\"
      }
    },
    $scst_common_apply
  }" \
  "targetluns:iqn.2026-06.example:scst.root:rescan" \
  '["scstadmin", "-resync_dev", "_dev_zvol_tank_root"]' \
  0 4 3 102

host_lun_rescan_tools="$tmpdir/fake-host-lun-rescan-tools"
mkdir -p "$host_lun_rescan_tools"
host_lun_rescan_disk_nix="$(command -v "$disk_nix_bin")"
host_lun_rescan_real_sh="$(command -v sh)"

cat > "$host_lun_rescan_tools/iscsiadm" <<'EOF'
#!/usr/bin/env bash
printf 'rescan ok\n'
EOF

cat > "$host_lun_rescan_tools/lsscsi" <<'EOF'
#!/usr/bin/env bash
printf '[0:0:0:0] disk fake target /dev/sda 1GiB\n'
EOF

cat > "$host_lun_rescan_tools/multipath" <<'EOF'
#!/usr/bin/env bash
printf 'reload ok\n'
EOF

cat > "$host_lun_rescan_tools/blockdev" <<'EOF'
#!/usr/bin/env bash
printf '1073741824\n'
EOF

cat > "$host_lun_rescan_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$host_lun_rescan_disk_nix" "\$@"
EOF

cat > "$host_lun_rescan_tools/sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" == "$host_lun_rescan_real_sh" || "\${1:-}" == "/bin/sh" ]]; then
  shift
fi
case "\$*" in
*"disk-nix-scsi-rescan"*)
  echo "synthetic host-side LUN SCSI rescan failure for disk-nix recovery coverage" >&2
  exit 94
  ;;
esac
exec "$host_lun_rescan_real_sh" "\$@"
EOF

chmod +x "$host_lun_rescan_tools/iscsiadm" "$host_lun_rescan_tools/lsscsi" \
  "$host_lun_rescan_tools/multipath" "$host_lun_rescan_tools/blockdev" \
  "$host_lun_rescan_tools/disk-nix" "$host_lun_rescan_tools/sh"

host_lun_rescan_spec="$tmpdir/host-lun-rescan-spec.json"
host_lun_rescan_json="$tmpdir/host-lun-rescan-apply.json"
host_lun_rescan_report="$tmpdir/host-lun-rescan-report.json"
host_lun_rescan_receipt="$tmpdir/host-lun-rescan-receipt.json"
host_lun_rescan_device="/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage-lun-0"
# shellcheck disable=SC2016
host_lun_rescan_sh='block=$(basename "$(readlink -f "$1")"); printf '\''1\n'\'' > "/sys/class/block/${block}/device/rescan"'

jq -n --arg device "$host_lun_rescan_device" '{
  luns: {
    "iqn.2026-06.example:storage/root:0": {
      operation: "rescan",
      devices: [$device]
    }
  }
}' > "$host_lun_rescan_spec"

if PATH="$host_lun_rescan_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$host_lun_rescan_spec" \
  --execute \
  --report-out "$host_lun_rescan_report" \
  --receipt-out "$host_lun_rescan_receipt" \
  --json > "$host_lun_rescan_json"; then
  echo "expected synthetic host-side LUN rescan failure to fail apply" >&2
  exit 1
fi

jq -e --arg device "$host_lun_rescan_device" --arg shcmd "$host_lun_rescan_sh" '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 1
  and .commandSummary.commandCount == 6
  and .commandSummary.mutatingCount == 3
  and .commandSummary.manualReviewCount == 1
  and .commandSummary.readyCount == 6
  and (.executionResults | length) == 4
  and .executionResults[0].success == true
  and .executionResults[0].actionId == "luns:iqn.2026-06.example:storage/root:0:rescan"
  and .executionResults[0].argv == ["iscsiadm", "--mode", "session", "--rescan"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["lsscsi", "-t", "-s"]
  and .executionResults[2].success == true
  and .executionResults[2].argv == ["disk-nix", "inspect", "iqn.2026-06.example:storage/root:0"]
  and .executionResults[3].success == false
  and .executionResults[3].statusCode == 94
  and .executionResults[3].actionId == "luns:iqn.2026-06.example:storage/root:0:rescan"
  and .executionResults[3].argv == ["sh", "-c", $shcmd, "disk-nix-scsi-rescan", $device]
  and (.executionResults[3].stderr | contains("synthetic host-side LUN SCSI rescan failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "luns:iqn.2026-06.example:storage/root:0:rescan"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["sh", "-c", $shcmd, "disk-nix-scsi-rescan", $device]
  and .partialExecutionRecovery.retryReviewActionIds == ["luns:iqn.2026-06.example:storage/root:0:rescan"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(.kind == "review-execution-failure"))
  and (.recoveryActions | any(
    .kind == "inspect-current-state"
    and (.commands | any(.argv == ["disk-nix", "probe-status", "--json"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "resume-after-fix"))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$host_lun_rescan_json" >/dev/null

cmp "$host_lun_rescan_json" "$host_lun_rescan_report" >/dev/null
jq -e --arg device "$host_lun_rescan_device" --arg shcmd "$host_lun_rescan_sh" '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "luns:iqn.2026-06.example:storage/root:0:rescan"
  and .report.partialExecutionRecovery.failedCommand == ["sh", "-c", $shcmd, "disk-nix-scsi-rescan", $device]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 1
' "$host_lun_rescan_receipt" >/dev/null

run_multipath_failure_case() {
  local name="$1"
  local spec_json="$2"
  local failed_action="$3"
  local failed_command_json="$4"
  local fail_match="$5"
  local fail_tool="$6"
  local status_code="$7"
  local failure_text="$8"

  local tools="$tmpdir/fake-multipath-$name-tools"
  mkdir -p "$tools"

  cat > "$tools/multipath" <<'EOF'
#!/usr/bin/env bash
if [[ "${DISK_NIX_MULTIPATH_FAIL_TOOL:-}" == "multipath" && "$*" == "${DISK_NIX_MULTIPATH_FAIL_MATCH:-}" ]]; then
  echo "${DISK_NIX_MULTIPATH_FAILURE_TEXT:-synthetic multipath failure}" >&2
  exit "${DISK_NIX_MULTIPATH_STATUS:-92}"
fi
printf '{}\n'
EOF

  cat > "$tools/multipathd" <<'EOF'
#!/usr/bin/env bash
if [[ "${DISK_NIX_MULTIPATH_FAIL_TOOL:-}" == "multipathd" && "$*" == "${DISK_NIX_MULTIPATH_FAIL_MATCH:-}" ]]; then
  echo "${DISK_NIX_MULTIPATH_FAILURE_TEXT:-synthetic multipath failure}" >&2
  exit "${DISK_NIX_MULTIPATH_STATUS:-92}"
fi
printf '{}\n'
EOF

  cat > "$tools/lsscsi" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

  chmod +x "$tools/multipath" "$tools/multipathd" "$tools/lsscsi"

  local spec="$tmpdir/multipath-$name-spec.json"
  local json="$tmpdir/multipath-$name-apply.json"
  local report="$tmpdir/multipath-$name-report.json"
  local receipt="$tmpdir/multipath-$name-receipt.json"

  jq -n "$spec_json" > "$spec"

  if DISK_NIX_MULTIPATH_FAIL_TOOL="$fail_tool" \
    DISK_NIX_MULTIPATH_FAIL_MATCH="$fail_match" \
    DISK_NIX_MULTIPATH_STATUS="$status_code" \
    DISK_NIX_MULTIPATH_FAILURE_TEXT="$failure_text" \
    PATH="$tools:$PATH" "$disk_nix_bin" apply \
      --spec "$spec" \
      --execute \
      --report-out "$report" \
      --receipt-out "$receipt" \
      --json > "$json"; then
    echo "expected synthetic multipath $name failure to fail apply" >&2
    exit 1
  fi

  jq -e \
    --arg action "$failed_action" \
    --arg text "$failure_text" \
    --argjson failed "$failed_command_json" \
    --argjson code "$status_code" '
    .status == "failed"
    and .apply.blockedCount == 0
    and .commandSummary.commandCount == 2
    and (.executionResults | length) == 2
    and .executionResults[0].success == true
    and .executionResults[0].argv == ["multipath", "-ll", "/dev/mapper/mpatha"]
    and .executionResults[1].success == false
    and .executionResults[1].statusCode == $code
    and .executionResults[1].argv == $failed
    and (.executionResults[1].stderr | contains($text))
    and .partialExecutionRecovery.completedActionIds == []
    and .partialExecutionRecovery.failedActionId == $action
    and .partialExecutionRecovery.failedPhase == "command"
    and .partialExecutionRecovery.failedCommand == $failed
    and .partialExecutionRecovery.retryReviewActionIds == [$action]
    and .partialExecutionRecovery.remainingActionIds == []
    and .partialExecutionRecovery.completedMutatingCommandCount == 0
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
  ' "$json" >/dev/null

  cmp "$json" "$report" >/dev/null
  jq -e \
    --arg action "$failed_action" \
    --argjson failed "$failed_command_json" '
    .receiptVersion == 1
    and .command == "apply"
    and .executeRequested == true
    and .report.status == "failed"
    and .report.partialExecutionRecovery.failedActionId == $action
    and .report.partialExecutionRecovery.failedCommand == $failed
    and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
  ' "$receipt" >/dev/null
}

run_multipath_failure_case \
  "add" \
  '{
    multipathMaps: {
      "root-map": {
        device: "/dev/mapper/mpatha",
        addDevices: ["/dev/sdb"]
      }
    },
    apply: {
      allowOffline: true
    }
  }' \
  "multipathMaps:root-map:add-device:/dev/sdb" \
  '["multipathd", "add", "path", "/dev/sdb"]' \
  "add path /dev/sdb" \
  "multipathd" \
  92 \
  "synthetic multipath add failure for disk-nix recovery coverage"

run_multipath_failure_case \
  "remove" \
  '{
    multipathMaps: {
      "root-map": {
        device: "/dev/mapper/mpatha",
        removeDevices: ["/dev/sde"]
      }
    },
    apply: {
      allowOffline: true,
      allowDeviceReplacement: true,
      allowPotentialDataLoss: true,
      allowDestructive: true,
      backupVerified: true
    }
  }' \
  "multipathMaps:root-map:remove-device:/dev/sde" \
  '["multipathd", "del", "path", "/dev/sde"]' \
  "del path /dev/sde" \
  "multipathd" \
  93 \
  "synthetic multipath remove failure for disk-nix recovery coverage"

run_multipath_failure_case \
  "destroy" \
  '{
    multipathMaps: {
      "root-map": {
        device: "/dev/mapper/mpatha",
        destroy: true
      }
    },
    apply: {
      allowOffline: true,
      allowDestructive: true
    }
  }' \
  "multipathmaps:root-map:destroy" \
  '["multipath", "-f", "/dev/mapper/mpatha"]' \
  "-f /dev/mapper/mpatha" \
  "multipath" \
  94 \
  "synthetic multipath destroy flush failure for disk-nix recovery coverage"

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
