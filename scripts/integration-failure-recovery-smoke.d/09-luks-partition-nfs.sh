luks_grow_tools="$tmpdir/fake-luks-grow-tools"
mkdir -p "$luks_grow_tools"

cat > "$luks_grow_tools/cryptsetup" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "resize cryptroot" ]]; then
  echo "synthetic LUKS grow failure for disk-nix recovery coverage" >&2
  exit 89
fi
printf '{}\n'
EOF

chmod +x "$luks_grow_tools/cryptsetup"

luks_grow_spec="$tmpdir/luks-grow-spec.json"
luks_grow_json="$tmpdir/luks-grow-apply.json"
luks_grow_report="$tmpdir/luks-grow-report.json"
luks_grow_receipt="$tmpdir/luks-grow-receipt.json"

jq -n '{
  luks: {
    devices: {
      cryptroot: {
        name: "cryptroot",
        device: "/dev/disk/by-partuuid/root",
        operation: "grow"
      }
    }
  },
  apply: {
    allowOffline: true,
    allowGrow: true
  }
}' > "$luks_grow_spec"

if PATH="$luks_grow_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$luks_grow_spec" \
  --execute \
  --report-out "$luks_grow_report" \
  --receipt-out "$luks_grow_receipt" \
  --json > "$luks_grow_json"; then
  echo "expected synthetic LUKS grow failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 3
  and (.executionResults | length) == 3
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["disk-nix", "inspect", "/dev/disk/by-partuuid/root"]
  and .executionResults[1].success == true
  and .executionResults[1].argv == ["cryptsetup", "status", "cryptroot"]
  and .executionResults[2].success == false
  and .executionResults[2].statusCode == 89
  and .executionResults[2].argv == ["cryptsetup", "resize", "cryptroot"]
  and (.executionResults[2].stderr | contains("synthetic LUKS grow failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "luks.devices:cryptroot:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["cryptsetup", "resize", "cryptroot"]
  and .partialExecutionRecovery.retryReviewActionIds == ["luks.devices:cryptroot:grow"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["cryptsetup", "status", "cryptroot"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "cryptroot", "--json"]))
    and (.notes | any(contains("LUKS changes")))
    and (.notes | any(contains("dependent consumers")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["cryptsetup", "status", "cryptroot"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "cryptroot", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["cryptsetup", "status", "cryptroot"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "cryptroot", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$luks_grow_json" >/dev/null

cmp "$luks_grow_json" "$luks_grow_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "luks.devices:cryptroot:grow"
  and .report.partialExecutionRecovery.failedCommand == ["cryptsetup", "resize", "cryptroot"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$luks_grow_receipt" >/dev/null

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

luks_token_remove_tools="$tmpdir/fake-luks-token-remove-tools"
mkdir -p "$luks_token_remove_tools"

cat > "$luks_token_remove_tools/cryptsetup" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "token remove --token-id 9 /dev/disk/by-id/root-luks" ]]; then
  echo "synthetic LUKS token remove failure for disk-nix recovery coverage" >&2
  exit 88
fi
printf '{}\n'
EOF

chmod +x "$luks_token_remove_tools/cryptsetup"

luks_token_remove_spec="$tmpdir/luks-token-remove-spec.json"
luks_token_remove_json="$tmpdir/luks-token-remove-apply.json"
luks_token_remove_report="$tmpdir/luks-token-remove-report.json"
luks_token_remove_receipt="$tmpdir/luks-token-remove-receipt.json"

jq -n '{
  luksTokens: {
    rootRemove: {
      operation: "remove-token",
      device: "/dev/disk/by-id/root-luks",
      token: "9"
    }
  },
  apply: {
    allowOffline: true,
    allowPotentialDataLoss: true,
    requireBackup: false,
    requireConfirmation: false
  }
}' > "$luks_token_remove_spec"

if PATH="$luks_token_remove_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$luks_token_remove_spec" \
  --execute \
  --report-out "$luks_token_remove_report" \
  --receipt-out "$luks_token_remove_receipt" \
  --json > "$luks_token_remove_json"; then
  echo "expected synthetic LUKS token remove failure to fail apply" >&2
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
  and .executionResults[1].statusCode == 88
  and .executionResults[1].argv == ["cryptsetup", "token", "remove", "--token-id", "9", "/dev/disk/by-id/root-luks"]
  and (.executionResults[1].stderr | contains("synthetic LUKS token remove failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "lukstokens:rootremove:remove-token"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["cryptsetup", "token", "remove", "--token-id", "9", "/dev/disk/by-id/root-luks"]
  and .partialExecutionRecovery.retryReviewActionIds == ["lukstokens:rootremove:remove-token"]
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
' "$luks_token_remove_json" >/dev/null

cmp "$luks_token_remove_json" "$luks_token_remove_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "lukstokens:rootremove:remove-token"
  and .report.partialExecutionRecovery.failedCommand == ["cryptsetup", "token", "remove", "--token-id", "9", "/dev/disk/by-id/root-luks"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$luks_token_remove_receipt" >/dev/null

luks_property_tools="$tmpdir/fake-luks-property-tools"
mkdir -p "$luks_property_tools"

cat > "$luks_property_tools/cryptsetup" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "config /dev/disk/by-id/root-luks --label root-new" ]]; then
  echo "synthetic LUKS property failure for disk-nix recovery coverage" >&2
  exit 89
fi
printf '{}\n'
EOF

chmod +x "$luks_property_tools/cryptsetup"

luks_property_spec="$tmpdir/luks-property-spec.json"
luks_property_json="$tmpdir/luks-property-apply.json"
luks_property_report="$tmpdir/luks-property-report.json"
luks_property_receipt="$tmpdir/luks-property-receipt.json"

jq -n '{
  luks: {
    devices: {
      cryptroot: {
        name: "cryptroot",
        device: "/dev/disk/by-id/root-luks",
        properties: {
          label: "root-new"
        }
      }
    }
  },
  apply: {
    allowOffline: true,
    allowPropertyChanges: true
  }
}' > "$luks_property_spec"

if PATH="$luks_property_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$luks_property_spec" \
  --execute \
  --report-out "$luks_property_report" \
  --receipt-out "$luks_property_receipt" \
  --json > "$luks_property_json"; then
  echo "expected synthetic LUKS property failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["disk-nix", "inspect", "cryptroot"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 89
  and .executionResults[1].argv == ["cryptsetup", "config", "/dev/disk/by-id/root-luks", "--label", "root-new"]
  and (.executionResults[1].stderr | contains("synthetic LUKS property failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "luks.devices:cryptroot:set-property:label"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["cryptsetup", "config", "/dev/disk/by-id/root-luks", "--label", "root-new"]
  and .partialExecutionRecovery.retryReviewActionIds == ["luks.devices:cryptroot:set-property:label"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["cryptsetup", "luksDump", "/dev/disk/by-id/root-luks"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/dev/disk/by-id/root-luks", "--json"]))
    and (.notes | any(contains("LUKS changes")))
    and (.notes | any(contains("header metadata")))
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
' "$luks_property_json" >/dev/null

cmp "$luks_property_json" "$luks_property_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "luks.devices:cryptroot:set-property:label"
  and .report.partialExecutionRecovery.failedCommand == ["cryptsetup", "config", "/dev/disk/by-id/root-luks", "--label", "root-new"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$luks_property_receipt" >/dev/null

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
