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
