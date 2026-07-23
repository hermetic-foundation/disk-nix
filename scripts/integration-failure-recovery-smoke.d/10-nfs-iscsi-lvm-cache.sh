nfs_remount_tools="$tmpdir/fake-nfs-remount-tools"
mkdir -p "$nfs_remount_tools"

cat > "$nfs_remount_tools/findmnt" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$nfs_remount_tools/nfsstat" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$nfs_remount_tools/mount" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "-o remount,_netdev,ro,vers=4.2 /srv/tuned" ]]; then
  echo "synthetic NFS remount failure for disk-nix recovery coverage" >&2
  exit 80
fi
printf '{}\n'
EOF

chmod +x "$nfs_remount_tools/findmnt" "$nfs_remount_tools/nfsstat" "$nfs_remount_tools/mount"

nfs_remount_spec="$tmpdir/nfs-remount-spec.json"
nfs_remount_json="$tmpdir/nfs-remount-apply.json"
nfs_remount_report="$tmpdir/nfs-remount-report.json"
nfs_remount_receipt="$tmpdir/nfs-remount-receipt.json"

jq -n '{
  nfs: {
    mounts: {
      "/srv/tuned": {
        operation: "remount",
        source: "nas.example.com:/srv/tuned",
        options: ["_netdev", "ro", "vers=4.2"]
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$nfs_remount_spec"

if PATH="$nfs_remount_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$nfs_remount_spec" \
  --execute \
  --report-out "$nfs_remount_report" \
  --receipt-out "$nfs_remount_receipt" \
  --json > "$nfs_remount_json"; then
  echo "expected synthetic NFS remount failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["findmnt", "--json", "/srv/tuned"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 80
  and .executionResults[1].argv == ["mount", "-o", "remount,_netdev,ro,vers=4.2", "/srv/tuned"]
  and (.executionResults[1].stderr | contains("synthetic NFS remount failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "nfs.mounts:/srv/tuned:remount"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["mount", "-o", "remount,_netdev,ro,vers=4.2", "/srv/tuned"]
  and .partialExecutionRecovery.retryReviewActionIds == ["nfs.mounts:/srv/tuned:remount"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["findmnt", "--json", "/srv/tuned"]))
    and (.commands | any(.argv == ["nfsstat", "-m", "/srv/tuned"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/srv/tuned", "--json"]))
    and (.notes | any(contains("NFS changes")))
    and (.notes | any(contains("negotiated mount options")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["nfsstat", "-m", "/srv/tuned"]))
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["findmnt", "--json", "/srv/tuned"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$nfs_remount_json" >/dev/null

cmp "$nfs_remount_json" "$nfs_remount_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "nfs.mounts:/srv/tuned:remount"
  and .report.partialExecutionRecovery.failedCommand == ["mount", "-o", "remount,_netdev,ro,vers=4.2", "/srv/tuned"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$nfs_remount_receipt" >/dev/null

nfs_unmount_tools="$tmpdir/fake-nfs-unmount-tools"
mkdir -p "$nfs_unmount_tools"

cat > "$nfs_unmount_tools/findmnt" <<'EOF'
#!/usr/bin/env bash
printf '{}\n'
EOF

cat > "$nfs_unmount_tools/umount" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "/srv/old" ]]; then
  echo "synthetic NFS unmount failure for disk-nix recovery coverage" >&2
  exit 91
fi
printf '{}\n'
EOF

chmod +x "$nfs_unmount_tools/findmnt" "$nfs_unmount_tools/umount"

nfs_unmount_spec="$tmpdir/nfs-unmount-spec.json"
nfs_unmount_json="$tmpdir/nfs-unmount-apply.json"
nfs_unmount_report="$tmpdir/nfs-unmount-report.json"
nfs_unmount_receipt="$tmpdir/nfs-unmount-receipt.json"

jq -n '{
  nfs: {
    mounts: {
      "/srv/old": {
        operation: "unmount",
        source: "nas.example.com:/srv/old"
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$nfs_unmount_spec"

if PATH="$nfs_unmount_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$nfs_unmount_spec" \
  --execute \
  --report-out "$nfs_unmount_report" \
  --receipt-out "$nfs_unmount_receipt" \
  --json > "$nfs_unmount_json"; then
  echo "expected synthetic NFS unmount failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 2
  and (.executionResults | length) == 2
  and .executionResults[0].success == true
  and .executionResults[0].argv == ["findmnt", "--json", "/srv/old"]
  and .executionResults[1].success == false
  and .executionResults[1].statusCode == 91
  and .executionResults[1].argv == ["umount", "/srv/old"]
  and (.executionResults[1].stderr | contains("synthetic NFS unmount failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "nfs.mounts:/srv/old:unmount"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["umount", "/srv/old"]
  and .partialExecutionRecovery.retryReviewActionIds == ["nfs.mounts:/srv/old:unmount"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["findmnt", "--json", "/srv/old"]))
    and (.commands | any(.argv == ["nfsstat", "-m", "/srv/old"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/srv/old", "--json"]))
    and (.notes | any(contains("NFS changes")))
    and (.notes | any(contains("dependent services")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["findmnt", "--json", "/srv/old"]))
    and (.commands | any(.argv == ["disk-nix", "topology", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["findmnt", "--json", "/srv/old"]))
    and (.commands | any(.argv == ["nfsstat", "-m", "/srv/old"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$nfs_unmount_json" >/dev/null

cmp "$nfs_unmount_json" "$nfs_unmount_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "nfs.mounts:/srv/old:unmount"
  and .report.partialExecutionRecovery.failedCommand == ["umount", "/srv/old"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$nfs_unmount_receipt" >/dev/null

nfs_export_tools="$tmpdir/fake-nfs-export-tools"
mkdir -p "$nfs_export_tools"

cat > "$nfs_export_tools/exportfs" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "-i -o rw,sync,no_subtree_check 192.0.2.0/24:/srv/share" ]]; then
  echo "synthetic NFS export failure for disk-nix recovery coverage" >&2
  exit 82
fi
printf '{}\n'
EOF

chmod +x "$nfs_export_tools/exportfs"

nfs_export_spec="$tmpdir/nfs-export-spec.json"
nfs_export_json="$tmpdir/nfs-export-apply.json"
nfs_export_report="$tmpdir/nfs-export-report.json"
nfs_export_receipt="$tmpdir/nfs-export-receipt.json"

jq -n '{
  exports: {
    share: {
      operation: "export",
      path: "/srv/share",
      client: "192.0.2.0/24",
      options: ["rw", "sync", "no_subtree_check"]
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$nfs_export_spec"

if PATH="$nfs_export_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$nfs_export_spec" \
  --execute \
  --report-out "$nfs_export_report" \
  --receipt-out "$nfs_export_receipt" \
  --json > "$nfs_export_json"; then
  echo "expected synthetic NFS export failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 1
  and (.executionResults | length) == 1
  and .executionResults[0].success == false
  and .executionResults[0].statusCode == 82
  and .executionResults[0].argv == ["exportfs", "-i", "-o", "rw,sync,no_subtree_check", "192.0.2.0/24:/srv/share"]
  and (.executionResults[0].stderr | contains("synthetic NFS export failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "exports:share:export"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["exportfs", "-i", "-o", "rw,sync,no_subtree_check", "192.0.2.0/24:/srv/share"]
  and .partialExecutionRecovery.retryReviewActionIds == ["exports:share:export"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["exportfs", "-v"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/srv/share", "--json"]))
    and (.notes | any(contains("NFS changes")))
    and (.notes | any(contains("exported paths")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["exportfs", "-v"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/srv/share", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["exportfs", "-v"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/srv/share", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$nfs_export_json" >/dev/null

cmp "$nfs_export_json" "$nfs_export_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "exports:share:export"
  and .report.partialExecutionRecovery.failedCommand == ["exportfs", "-i", "-o", "rw,sync,no_subtree_check", "192.0.2.0/24:/srv/share"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$nfs_export_receipt" >/dev/null

nfs_unexport_tools="$tmpdir/fake-nfs-unexport-tools"
mkdir -p "$nfs_unexport_tools"

cat > "$nfs_unexport_tools/exportfs" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "-u 192.0.2.55:/srv/old" ]]; then
  echo "synthetic NFS unexport failure for disk-nix recovery coverage" >&2
  exit 83
fi
printf '{}\n'
EOF

chmod +x "$nfs_unexport_tools/exportfs"

nfs_unexport_spec="$tmpdir/nfs-unexport-spec.json"
nfs_unexport_json="$tmpdir/nfs-unexport-apply.json"
nfs_unexport_report="$tmpdir/nfs-unexport-report.json"
nfs_unexport_receipt="$tmpdir/nfs-unexport-receipt.json"

jq -n '{
  exports: {
    oldshare: {
      operation: "unexport",
      path: "/srv/old",
      client: "192.0.2.55"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$nfs_unexport_spec"

if PATH="$nfs_unexport_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$nfs_unexport_spec" \
  --execute \
  --report-out "$nfs_unexport_report" \
  --receipt-out "$nfs_unexport_receipt" \
  --json > "$nfs_unexport_json"; then
  echo "expected synthetic NFS unexport failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.commandCount == 1
  and (.executionResults | length) == 1
  and .executionResults[0].success == false
  and .executionResults[0].statusCode == 83
  and .executionResults[0].argv == ["exportfs", "-u", "192.0.2.55:/srv/old"]
  and (.executionResults[0].stderr | contains("synthetic NFS unexport failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "exports:oldshare:unexport"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["exportfs", "-u", "192.0.2.55:/srv/old"]
  and .partialExecutionRecovery.retryReviewActionIds == ["exports:oldshare:unexport"]
  and .partialExecutionRecovery.remainingActionIds == []
  and .partialExecutionRecovery.completedMutatingCommandCount == 0
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(
    .kind == "domain-recovery"
    and (.commands | any(.argv == ["exportfs", "-v"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/srv/old", "--json"]))
    and (.notes | any(contains("NFS changes")))
    and (.notes | any(contains("dependent services")))
  ))
  and (.recoveryActions | any(
    .kind == "roll-forward-review"
    and (.commands | any(.argv == ["disk-nix", "apply", "--spec", "<spec>", "--probe-current", "--json"] and .readiness == "manual-only"))
    and (.commands | any(.argv == ["exportfs", "-v"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/srv/old", "--json"]))
  ))
  and (.recoveryActions | any(
    .kind == "rollback-review"
    and (.commands | all(.mutates == false))
    and (.commands | any(.argv == ["exportfs", "-v"]))
    and (.commands | any(.argv == ["disk-nix", "inspect", "/srv/old", "--json"]))
  ))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$nfs_unexport_json" >/dev/null

cmp "$nfs_unexport_json" "$nfs_unexport_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "exports:oldshare:unexport"
  and .report.partialExecutionRecovery.failedCommand == ["exportfs", "-u", "192.0.2.55:/srv/old"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$nfs_unexport_receipt" >/dev/null

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

iscsi_rescan_tools="$tmpdir/fake-iscsi-rescan-tools"
mkdir -p "$iscsi_rescan_tools"
iscsi_rescan_disk_nix="$(command -v "$disk_nix_bin")"

cat > "$iscsi_rescan_tools/iscsiadm" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == "--mode session --rescan" ]]; then
  echo "synthetic iscsi rescan failure for disk-nix recovery coverage" >&2
  exit 93
fi
printf 'tcp: [1] 192.0.2.10:3260,1 iqn.2026-06.example:storage.root\n'
EOF

cat > "$iscsi_rescan_tools/lsscsi" <<'EOF'
#!/usr/bin/env bash
printf '[0:0:0:0] disk fake target /dev/sda 1GiB\n'
EOF

cat > "$iscsi_rescan_tools/disk-nix" <<EOF
#!/usr/bin/env bash
exec "$iscsi_rescan_disk_nix" "\$@"
EOF

chmod +x "$iscsi_rescan_tools/iscsiadm" "$iscsi_rescan_tools/lsscsi" "$iscsi_rescan_tools/disk-nix"

iscsi_rescan_spec="$tmpdir/iscsi-rescan-spec.json"
iscsi_rescan_json="$tmpdir/iscsi-rescan-apply.json"
iscsi_rescan_report="$tmpdir/iscsi-rescan-report.json"
iscsi_rescan_receipt="$tmpdir/iscsi-rescan-receipt.json"

jq -n '{
  iscsiSessions: {
    "iqn.2026-06.example:storage.root": {
      operation: "rescan"
    }
  }
}' > "$iscsi_rescan_spec"

if PATH="$iscsi_rescan_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$iscsi_rescan_spec" \
  --execute \
  --report-out "$iscsi_rescan_report" \
  --receipt-out "$iscsi_rescan_receipt" \
  --json > "$iscsi_rescan_json"; then
  echo "expected synthetic iSCSI rescan failure to fail apply" >&2
  exit 1
fi

jq -e '
  .status == "failed"
  and .apply.blockedCount == 0
  and .commandSummary.stepCount == 1
  and .commandSummary.commandCount == 3
  and .commandSummary.mutatingCount == 1
  and .commandSummary.manualReviewCount == 1
  and .commandSummary.readyCount == 3
  and (.executionResults | length) == 1
  and .executionResults[0].success == false
  and .executionResults[0].statusCode == 93
  and .executionResults[0].actionId == "iscsisessions:iqn.2026-06.example:storage.root:rescan"
  and .executionResults[0].argv == ["iscsiadm", "--mode", "session", "--rescan"]
  and (.executionResults[0].stderr | contains("synthetic iscsi rescan failure"))
  and .partialExecutionRecovery.completedActionIds == []
  and .partialExecutionRecovery.failedActionId == "iscsisessions:iqn.2026-06.example:storage.root:rescan"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["iscsiadm", "--mode", "session", "--rescan"]
  and .partialExecutionRecovery.retryReviewActionIds == ["iscsisessions:iqn.2026-06.example:storage.root:rescan"]
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
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$iscsi_rescan_json" >/dev/null

cmp "$iscsi_rescan_json" "$iscsi_rescan_report" >/dev/null
jq -e '
  .receiptVersion == 1
  and .command == "apply"
  and .executeRequested == true
  and .report.status == "failed"
  and .report.partialExecutionRecovery.failedActionId == "iscsisessions:iqn.2026-06.example:storage.root:rescan"
  and .report.partialExecutionRecovery.failedCommand == ["iscsiadm", "--mode", "session", "--rescan"]
  and .report.partialExecutionRecovery.completedMutatingCommandCount == 0
' "$iscsi_rescan_receipt" >/dev/null

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
