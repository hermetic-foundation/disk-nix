target_lun_lio_grow_tools="$tmpdir/fake-target-lun-lio-grow-tools"
mkdir -p "$target_lun_lio_grow_tools"

cat > "$target_lun_lio_grow_tools/targetcli" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$target_lun_lio_grow_tools/blockdev" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--getsize64 /dev/zvol/tank/root" ]]; then
  printf '4398046511104\n'
  exit 0
fi
echo "unexpected blockdev invocation: $*" >&2
exit 87
EOF

cat > "$target_lun_lio_grow_tools/lsscsi" <<'EOF'
#!/usr/bin/env bash
printf '[0:0:0:7] disk LIO-ORG ROOT 4TiB /dev/sdz\n'
exit 0
EOF

cat > "$target_lun_lio_grow_tools/multipath" <<'EOF'
#!/usr/bin/env bash
printf 'mpathroot (36001405root) dm-9 LIO-ORG,ROOT\n'
exit 0
EOF

cat > "$target_lun_lio_grow_tools/disk-nix" <<'EOF'
#!/usr/bin/env bash
if [[ "$1" == "inspect" ]]; then
  printf '{"object":"%s","verified":true}\n' "$2"
  exit 0
fi
echo "unexpected disk-nix invocation: $*" >&2
exit 88
EOF

chmod +x "$target_lun_lio_grow_tools/targetcli" \
  "$target_lun_lio_grow_tools/blockdev" \
  "$target_lun_lio_grow_tools/lsscsi" \
  "$target_lun_lio_grow_tools/multipath" \
  "$target_lun_lio_grow_tools/disk-nix"

target_lun_lio_grow_spec="$tmpdir/target-lun-lio-grow-spec.json"
target_lun_lio_grow_json="$tmpdir/target-lun-lio-grow-apply.json"
target_lun_lio_grow_report="$tmpdir/target-lun-lio-grow-report.json"
target_lun_lio_grow_receipt="$tmpdir/target-lun-lio-grow-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:storage.root": {
        operation: "grow",
        provider: "lio",
        source: "/dev/zvol/tank/root",
        desiredSize: "4TiB",
        lun: 7,
        properties: {
          "lio.writeCache": "off"
        }
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$target_lun_lio_grow_spec"

PATH="$target_lun_lio_grow_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_lio_grow_spec" \
  --execute \
  --report-out "$target_lun_lio_grow_report" \
  --receipt-out "$target_lun_lio_grow_receipt" \
  --json > "$target_lun_lio_grow_json"

jq -e '
  .status == "succeeded"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 2
  and .commandSummary.commandCount == 12
  and .commandSummary.needsDomainImplementationCount == 0
  and (.executionResults | length) == 17
  and (.commandPlan | any(
    .actionId == "targetluns:iqn.2026-06.example:storage.root:grow"
    and (.commands | any(.argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"] and .mutates == false))
    and (.commands | any(.argv == ["targetcli", "/backstores/block/_dev_zvol_tank_root", "ls"] and .mutates == false))
    and (.commands | any(.argv == ["blockdev", "--getsize64", "/dev/zvol/tank/root"] and .mutates == false and .readiness == "ready"))
    and (.commands | any(
      .argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root/tpg1/luns", "ls"]
      and .mutates == false
      and .readiness == "ready"
    ))
    and (.commands | any(
      .argv == ["targetcli", "saveconfig"]
      and .mutates == true
      and .readiness == "ready"
    ))
  ))
  and (.commandPlan | any(
    .actionId == "targetLuns:iqn.2026-06.example:storage.root:set-property:lio.writeCache"
    and (.commands | any(.argv == ["targetcli", "/backstores/block/_dev_zvol_tank_root", "ls"] and .mutates == false))
    and (.commands | any(
      .argv == ["targetcli", "/backstores/block/_dev_zvol_tank_root", "set", "attribute", "emulate_write_cache=0"]
      and .mutates == true
      and .readiness == "ready"
    ))
    and (.commands | any(
      .argv == ["targetcli", "saveconfig"]
      and .mutates == true
      and .readiness == "ready"
    ))
  ))
  and (.verificationPlan | any(
    .actionId == "targetluns:iqn.2026-06.example:storage.root:grow"
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"] and .mutates == false))
    and (.commands | any(.argv == ["multipath", "-ll"] and .mutates == false))
    and (.commands | any(.argv == ["disk-nix", "inspect", "iqn.2026-06.example:storage.root", "--json"] and .mutates == false))
  ))
' "$target_lun_lio_grow_json" >/dev/null

cmp "$target_lun_lio_grow_json" "$target_lun_lio_grow_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "succeeded"
  and .report.commandSummary.needsDomainImplementationCount == 0
  and (.report.executionResults | length) == 17
' "$target_lun_lio_grow_receipt" >/dev/null

target_lun_lio_property_tools="$tmpdir/fake-target-lun-lio-property-tools"
mkdir -p "$target_lun_lio_property_tools"

cat > "$target_lun_lio_property_tools/targetcli" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/backstores/block/_dev_zvol_tank_root set attribute emulate_write_cache=0" ]]; then
  echo "synthetic target-side LUN LIO property failure for disk-nix recovery coverage" >&2
  exit 88
fi
printf '{}\n'
exit 0
EOF

chmod +x "$target_lun_lio_property_tools/targetcli"

target_lun_lio_property_spec="$tmpdir/target-lun-lio-property-spec.json"
target_lun_lio_property_json="$tmpdir/target-lun-lio-property-apply.json"
target_lun_lio_property_report="$tmpdir/target-lun-lio-property-report.json"
target_lun_lio_property_receipt="$tmpdir/target-lun-lio-property-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:storage.root": {
        provider: "lio",
        source: "/dev/zvol/tank/root",
        lun: 7,
        properties: {
          "lio.writeCache": "off"
        }
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$target_lun_lio_property_spec"

if PATH="$target_lun_lio_property_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_lio_property_spec" \
  --execute \
  --report-out "$target_lun_lio_property_report" \
  --receipt-out "$target_lun_lio_property_receipt" \
  --json > "$target_lun_lio_property_json"; then
  echo "expected synthetic target-side LUN LIO property failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 1
  and .commandSummary.commandCount == 5
  and .commandSummary.needsDomainImplementationCount == 0
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["targetcli", "/backstores/block/_dev_zvol_tank_root", "ls"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 88
  and .executionResults[2].argv == ["targetcli", "/backstores/block/_dev_zvol_tank_root", "set", "attribute", "emulate_write_cache=0"]
  and (.executionResults[2].stderr | contains("synthetic target-side LUN LIO property failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetLuns:iqn.2026-06.example:storage.root:set-property:lio.writeCache"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["targetcli", "/backstores/block/_dev_zvol_tank_root", "set", "attribute", "emulate_write_cache=0"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetLuns:iqn.2026-06.example:storage.root:set-property:lio.writeCache"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
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
    and (.commands | any(.argv == ["targetcli", "/iscsi", "ls"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$target_lun_lio_property_json" >/dev/null

cmp "$target_lun_lio_property_json" "$target_lun_lio_property_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetLuns:iqn.2026-06.example:storage.root:set-property:lio.writeCache"
  and .report.partialExecutionRecovery.failedCommand == ["targetcli", "/backstores/block/_dev_zvol_tank_root", "set", "attribute", "emulate_write_cache=0"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$target_lun_lio_property_receipt" >/dev/null

target_lun_lio_rescan_tools="$tmpdir/fake-target-lun-lio-rescan-tools"
mkdir -p "$target_lun_lio_rescan_tools"

cat > "$target_lun_lio_rescan_tools/targetcli" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/iscsi/iqn.2026-06.example:storage.root ls" ]]; then
  echo "synthetic target-side LUN LIO rescan inventory failure for disk-nix recovery coverage" >&2
  exit 90
fi
printf '{}\n'
exit 0
EOF

chmod +x "$target_lun_lio_rescan_tools/targetcli"

target_lun_lio_rescan_spec="$tmpdir/target-lun-lio-rescan-spec.json"
target_lun_lio_rescan_json="$tmpdir/target-lun-lio-rescan-apply.json"
target_lun_lio_rescan_report="$tmpdir/target-lun-lio-rescan-report.json"
target_lun_lio_rescan_receipt="$tmpdir/target-lun-lio-rescan-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:storage.root": {
        operation: "rescan",
        provider: "lio"
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$target_lun_lio_rescan_spec"

if PATH="$target_lun_lio_rescan_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_lio_rescan_spec" \
  --execute \
  --report-out "$target_lun_lio_rescan_report" \
  --receipt-out "$target_lun_lio_rescan_receipt" \
  --json > "$target_lun_lio_rescan_json"; then
  echo "expected synthetic target-side LUN LIO rescan failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 1
  and .commandSummary.commandCount == 2
  and .commandSummary.needsDomainImplementationCount == 0
  and (.executionResults | length) == 1
  and .executionResults[0].success == false
  and .executionResults[0].statusCode == 90
  and .executionResults[0].argv == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
  and (.executionResults[0].stderr | contains("synthetic target-side LUN LIO rescan inventory failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:storage.root:rescan"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetluns:iqn.2026-06.example:storage.root:rescan"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
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
    and (.commands | any(.argv == ["targetcli", "/iscsi", "ls"]))
  ))
' "$target_lun_lio_rescan_json" >/dev/null

cmp "$target_lun_lio_rescan_json" "$target_lun_lio_rescan_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:storage.root:rescan"
  and .report.partialExecutionRecovery.failedCommand == ["targetcli", "/iscsi/iqn.2026-06.example:storage.root", "ls"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$target_lun_lio_rescan_receipt" >/dev/null

target_lun_tgt_grow_tools="$tmpdir/fake-target-lun-tgt-grow-tools"
mkdir -p "$target_lun_tgt_grow_tools"

cat > "$target_lun_tgt_grow_tools/tgtadm" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
exit 0
EOF

cat > "$target_lun_tgt_grow_tools/blockdev" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--getsize64 /dev/zvol/tank/root" ]]; then
  printf '4398046511104\n'
  exit 0
fi
echo "unexpected blockdev invocation: $*" >&2
exit 87
EOF

cat > "$target_lun_tgt_grow_tools/tgt-admin" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--dump" ]]; then
  printf '<target iqn.2026-06.example:tgt.root>\n  backing-store /dev/zvol/tank/root\n</target>\n'
  exit 0
fi
echo "unexpected tgt-admin invocation: $*" >&2
exit 88
EOF

cat > "$target_lun_tgt_grow_tools/lsscsi" <<'EOF'
#!/usr/bin/env bash
printf '[0:0:0:8] disk TGT ROOT 4TiB /dev/sdy\n'
exit 0
EOF

cat > "$target_lun_tgt_grow_tools/multipath" <<'EOF'
#!/usr/bin/env bash
printf 'mpathtgt (36001405tgt) dm-8 TGT,ROOT\n'
exit 0
EOF

cat > "$target_lun_tgt_grow_tools/disk-nix" <<'EOF'
#!/usr/bin/env bash
if [[ "$1" == "inspect" ]]; then
  printf '{"object":"%s","verified":true}\n' "$2"
  exit 0
fi
echo "unexpected disk-nix invocation: $*" >&2
exit 89
EOF

chmod +x "$target_lun_tgt_grow_tools/tgtadm" \
  "$target_lun_tgt_grow_tools/blockdev" \
  "$target_lun_tgt_grow_tools/tgt-admin" \
  "$target_lun_tgt_grow_tools/lsscsi" \
  "$target_lun_tgt_grow_tools/multipath" \
  "$target_lun_tgt_grow_tools/disk-nix"

target_lun_tgt_grow_spec="$tmpdir/target-lun-tgt-grow-spec.json"
target_lun_tgt_grow_json="$tmpdir/target-lun-tgt-grow-apply.json"
target_lun_tgt_grow_report="$tmpdir/target-lun-tgt-grow-report.json"
target_lun_tgt_grow_receipt="$tmpdir/target-lun-tgt-grow-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:tgt.root": {
        operation: "grow",
        provider: "tgt",
        targetId: 42,
        source: "/dev/zvol/tank/root",
        desiredSize: "4TiB",
        lun: 8,
        properties: {
          "tgt.writeCache": "off"
        }
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$target_lun_tgt_grow_spec"

PATH="$target_lun_tgt_grow_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_tgt_grow_spec" \
  --execute \
  --report-out "$target_lun_tgt_grow_report" \
  --receipt-out "$target_lun_tgt_grow_receipt" \
  --json > "$target_lun_tgt_grow_json"

jq -e '
  .status == "succeeded"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 2
  and .commandSummary.commandCount == 9
  and .commandSummary.needsDomainImplementationCount == 0
  and (.executionResults | length) == 14
  and (.commandPlan | any(
    .actionId == "targetluns:iqn.2026-06.example:tgt.root:grow"
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"] and .mutates == false))
    and (.commands | any(.argv == ["blockdev", "--getsize64", "/dev/zvol/tank/root"] and .mutates == false and .readiness == "ready"))
    and (.commands | any(
      .argv == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "update", "--tid", "42", "--lun", "8", "--params", "online=1"]
      and .mutates == true
      and .readiness == "ready"
    ))
    and (.commands | any(
      .argv == ["tgt-admin", "--dump"]
      and .mutates == false
      and .readiness == "ready"
    ))
  ))
  and (.commandPlan | any(
    .actionId == "targetLuns:iqn.2026-06.example:tgt.root:set-property:tgt.writeCache"
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"] and .mutates == false))
    and (.commands | any(
      .argv == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "update", "--tid", "42", "--lun", "8", "--name", "tgt.writeCache", "--value", "off"]
      and .mutates == true
      and .readiness == "ready"
    ))
  ))
  and (.verificationPlan | any(
    .actionId == "targetluns:iqn.2026-06.example:tgt.root:grow"
    and (.commands | any(.argv == ["lsscsi", "-t", "-s"] and .mutates == false))
    and (.commands | any(.argv == ["multipath", "-ll"] and .mutates == false))
    and (.commands | any(.argv == ["disk-nix", "inspect", "iqn.2026-06.example:tgt.root", "--json"] and .mutates == false))
  ))
' "$target_lun_tgt_grow_json" >/dev/null

cmp "$target_lun_tgt_grow_json" "$target_lun_tgt_grow_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "succeeded"
  and .report.commandSummary.needsDomainImplementationCount == 0
  and (.report.executionResults | length) == 14
' "$target_lun_tgt_grow_receipt" >/dev/null

target_lun_tgt_property_tools="$tmpdir/fake-target-lun-tgt-property-tools"
mkdir -p "$target_lun_tgt_property_tools"

cat > "$target_lun_tgt_property_tools/tgtadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--lld iscsi --mode logicalunit --op update --tid 42 --lun 8 --name tgt.writeCache --value off" ]]; then
  echo "synthetic target-side LUN tgt property failure for disk-nix recovery coverage" >&2
  exit 89
fi
printf '{}\n'
exit 0
EOF

chmod +x "$target_lun_tgt_property_tools/tgtadm"

target_lun_tgt_property_spec="$tmpdir/target-lun-tgt-property-spec.json"
target_lun_tgt_property_json="$tmpdir/target-lun-tgt-property-apply.json"
target_lun_tgt_property_report="$tmpdir/target-lun-tgt-property-report.json"
target_lun_tgt_property_receipt="$tmpdir/target-lun-tgt-property-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:tgt.root": {
        provider: "tgt",
        targetId: 42,
        source: "/dev/zvol/tank/root",
        lun: 8,
        properties: {
          "tgt.writeCache": "off"
        }
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$target_lun_tgt_property_spec"

if PATH="$target_lun_tgt_property_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_tgt_property_spec" \
  --execute \
  --report-out "$target_lun_tgt_property_report" \
  --receipt-out "$target_lun_tgt_property_receipt" \
  --json > "$target_lun_tgt_property_json"; then
  echo "expected synthetic target-side LUN tgt property failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 1
  and .commandSummary.commandCount == 3
  and .commandSummary.needsDomainImplementationCount == 0
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 89
  and .executionResults[1].argv == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "update", "--tid", "42", "--lun", "8", "--name", "tgt.writeCache", "--value", "off"]
  and (.executionResults[1].stderr | contains("synthetic target-side LUN tgt property failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetLuns:iqn.2026-06.example:tgt.root:set-property:tgt.writeCache"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "update", "--tid", "42", "--lun", "8", "--name", "tgt.writeCache", "--value", "off"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetLuns:iqn.2026-06.example:tgt.root:set-property:tgt.writeCache"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
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
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$target_lun_tgt_property_json" >/dev/null

cmp "$target_lun_tgt_property_json" "$target_lun_tgt_property_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetLuns:iqn.2026-06.example:tgt.root:set-property:tgt.writeCache"
  and .report.partialExecutionRecovery.failedCommand == ["tgtadm", "--lld", "iscsi", "--mode", "logicalunit", "--op", "update", "--tid", "42", "--lun", "8", "--name", "tgt.writeCache", "--value", "off"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$target_lun_tgt_property_receipt" >/dev/null

target_lun_tgt_rescan_tools="$tmpdir/fake-target-lun-tgt-rescan-tools"
mkdir -p "$target_lun_tgt_rescan_tools"

cat > "$target_lun_tgt_rescan_tools/tgtadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--lld iscsi --mode target --op show --tid 42" ]]; then
  echo "synthetic target-side LUN tgt rescan inventory failure for disk-nix recovery coverage" >&2
  exit 91
fi
printf '{}\n'
exit 0
EOF

chmod +x "$target_lun_tgt_rescan_tools/tgtadm"

target_lun_tgt_rescan_spec="$tmpdir/target-lun-tgt-rescan-spec.json"
target_lun_tgt_rescan_json="$tmpdir/target-lun-tgt-rescan-apply.json"
target_lun_tgt_rescan_report="$tmpdir/target-lun-tgt-rescan-report.json"
target_lun_tgt_rescan_receipt="$tmpdir/target-lun-tgt-rescan-receipt.json"

jq -n '{
  spec: {
    targetLuns: {
      "iqn.2026-06.example:tgt.root": {
        operation: "rescan",
        provider: "tgt",
        targetId: 42
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$target_lun_tgt_rescan_spec"

if PATH="$target_lun_tgt_rescan_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$target_lun_tgt_rescan_spec" \
  --execute \
  --report-out "$target_lun_tgt_rescan_report" \
  --receipt-out "$target_lun_tgt_rescan_receipt" \
  --json > "$target_lun_tgt_rescan_json"; then
  echo "expected synthetic target-side LUN tgt rescan failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 1
  and .commandSummary.commandCount == 2
  and .commandSummary.needsDomainImplementationCount == 0
  and (.executionResults | length) == 1
  and .executionResults[0].success == false
  and .executionResults[0].statusCode == 91
  and .executionResults[0].argv == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"]
  and (.executionResults[0].stderr | contains("synthetic target-side LUN tgt rescan inventory failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:tgt.root:rescan"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"]
  and .partialExecutionRecovery.retryReviewActionIds == ["targetluns:iqn.2026-06.example:tgt.root:rescan"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
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
' "$target_lun_tgt_rescan_json" >/dev/null

cmp "$target_lun_tgt_rescan_json" "$target_lun_tgt_rescan_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "targetluns:iqn.2026-06.example:tgt.root:rescan"
  and .report.partialExecutionRecovery.failedCommand == ["tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show", "--tid", "42"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$target_lun_tgt_rescan_receipt" >/dev/null
