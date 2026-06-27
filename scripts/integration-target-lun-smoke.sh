#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run target-side LUN integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates
a temporary loop-backed LIO block backstore and iSCSI target/LUN mapping,
mutates a target-side LUN property, maps and unmaps a second target-side LUN,
verifies target-side LUN data survives a failed and resumed detach apply,
proves destructive target-side removal is refused without destructive policy,
then removes the temporary target state and backing files during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "target-side LUN integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"
target_iqn="${DISK_NIX_TARGET_LUN_IQN:-iqn.2026-06.example:disk-nix-lio-smoke}"
lun_id="${DISK_NIX_TARGET_LUN_ID:-7}"
attach_lun_id="${DISK_NIX_TARGET_LUN_ATTACH_ID:-8}"
initiator_iqn="${DISK_NIX_TARGET_LUN_INITIATOR:-iqn.2026-06.example:disk-nix-initiator}"

for tool in "$disk_nix_bin" cmp grep install jq losetup mkfs.ext4 modprobe mount mountpoint targetcli truncate umount; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loopdev=""
attach_loopdev=""
backstore=""
attach_backstore=""
target_created=0
backstore_created=0
attach_backstore_created=0
lun_mounted=0

cleanup() {
  if [[ "$lun_mounted" == "1" ]] && mountpoint -q "$tmpdir/lun-mnt"; then
    umount "$tmpdir/lun-mnt" || true
  fi
  if [[ "$target_created" == "1" ]]; then
    targetcli "/iscsi" delete "$target_iqn" >/dev/null 2>&1 || true
  fi
  if [[ "$backstore_created" == "1" ]] && [[ -n "$backstore" ]]; then
    targetcli "/backstores/block" delete "$backstore" >/dev/null 2>&1 || true
  fi
  if [[ "$attach_backstore_created" == "1" ]] && [[ -n "$attach_backstore" ]]; then
    targetcli "/backstores/block" delete "$attach_backstore" >/dev/null 2>&1 || true
  fi
  targetcli saveconfig >/dev/null 2>&1 || true
  if [[ -n "$loopdev" ]] && losetup --list "$loopdev" >/dev/null 2>&1; then
    losetup --detach "$loopdev" || true
  fi
  if [[ -n "$attach_loopdev" ]] && losetup --list "$attach_loopdev" >/dev/null 2>&1; then
    losetup --detach "$attach_loopdev" || true
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

modprobe target_core_mod
modprobe target_core_iblock || true

backing="$tmpdir/disk-nix-target-lun.img"
attach_backing="$tmpdir/disk-nix-target-lun-attach.img"
property_spec="$tmpdir/property-spec.json"
property_report="$tmpdir/property-report.json"
attach_spec="$tmpdir/attach-spec.json"
attach_report="$tmpdir/attach-report.json"
detach_spec="$tmpdir/detach-spec.json"
detach_report="$tmpdir/detach-report.json"
failed_detach_report="$tmpdir/failed-detach-report.json"
destroy_refusal_spec="$tmpdir/destroy-refusal-spec.json"
sentinel_expected="$tmpdir/target-lun-sentinel.expected"

truncate --size 64M "$backing"
truncate --size 64M "$attach_backing"
loopdev="$(losetup --find --show "$backing")"
attach_loopdev="$(losetup --find --show "$attach_backing")"
backstore="_${loopdev#/}"
backstore="${backstore//\//_}"
attach_backstore="_${attach_loopdev#/}"
attach_backstore="${attach_backstore//\//_}"

targetcli /backstores/block create "name=$backstore" "dev=$loopdev" >/dev/null
backstore_created=1
targetcli /backstores/block create "name=$attach_backstore" "dev=$attach_loopdev" >/dev/null
attach_backstore_created=1
targetcli /iscsi create "$target_iqn" >/dev/null
target_created=1
targetcli "/iscsi/$target_iqn/tpg1/luns" create "/backstores/block/$backstore" "lun=$lun_id" >/dev/null
targetcli saveconfig >/dev/null

jq -n \
  --arg target_iqn "$target_iqn" \
  --arg loopdev "$loopdev" \
  --argjson lun_id "$lun_id" \
  '{
    version: 1,
    targetLuns: {
      ($target_iqn): {
        provider: "lio",
        source: $loopdev,
        lun: $lun_id,
        properties: {
          "lio.writeCache": "off"
        }
      }
    },
    apply: {
      allowOffline: true
    }
  }' > "$property_spec"

if ! "$disk_nix_bin" apply \
  --spec "$property_spec" \
  --execute \
  --report-out "$property_report" \
  --json > "$tmpdir/property-apply.json"; then
  cat "$tmpdir/property-apply.json" >&2 || true
  cat "$property_report" >&2 || true
  exit 1
fi

jq -e \
  --arg target_iqn "$target_iqn" \
  --arg backstore "$backstore" \
  '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("targetLuns:" + $target_iqn + ":set-property:lio.writeCache"))
    | .commands | any(.argv == ["targetcli", ("/backstores/block/" + $backstore), "set", "attribute", "emulate_write_cache=0"]))
  and (.executionResults
    | any(.argv == ["targetcli", ("/backstores/block/" + $backstore), "set", "attribute", "emulate_write_cache=0"] and .success == true))
' "$tmpdir/property-apply.json" >/dev/null

cmp "$tmpdir/property-apply.json" "$property_report" >/dev/null
targetcli "/backstores/block/$backstore" ls >/dev/null

jq -n \
  --arg target_iqn "$target_iqn" \
  --arg attach_loopdev "$attach_loopdev" \
  --arg initiator_iqn "$initiator_iqn" \
  --argjson attach_lun_id "$attach_lun_id" \
  '{
    version: 1,
    targetLuns: {
      ($target_iqn): {
        operation: "attach",
        provider: "lio",
        source: $attach_loopdev,
        lun: $attach_lun_id,
        client: $initiator_iqn
      }
    },
    apply: {
      allowOffline: true
    }
  }' > "$attach_spec"

if ! "$disk_nix_bin" apply \
  --spec "$attach_spec" \
  --execute \
  --report-out "$attach_report" \
  --json > "$tmpdir/attach-apply.json"; then
  cat "$tmpdir/attach-apply.json" >&2 || true
  cat "$attach_report" >&2 || true
  exit 1
fi

jq -e \
  --arg target_iqn "$target_iqn" \
  --arg attach_backstore "$attach_backstore" \
  --arg initiator_iqn "$initiator_iqn" \
  --argjson attach_lun_id "$attach_lun_id" \
  '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("targetluns:" + $target_iqn + ":attach"))
    | .commands
    | any(.argv == ["targetcli", ("/iscsi/" + $target_iqn + "/tpg1/luns"), "create", ("/backstores/block/" + $attach_backstore), ("lun=" + ($attach_lun_id | tostring))])
    and any(.argv == ["targetcli", ("/iscsi/" + $target_iqn + "/tpg1/acls"), "create", $initiator_iqn]))
  and (.executionResults
    | any(.argv == ["targetcli", ("/iscsi/" + $target_iqn + "/tpg1/luns"), "create", ("/backstores/block/" + $attach_backstore), ("lun=" + ($attach_lun_id | tostring))] and .success == true))
' "$tmpdir/attach-apply.json" >/dev/null

cmp "$tmpdir/attach-apply.json" "$attach_report" >/dev/null
targetcli "/iscsi/$target_iqn/tpg1/luns" ls | grep -E "lun[[:space:]]*$attach_lun_id|lun$attach_lun_id" >/dev/null
targetcli "/iscsi/$target_iqn/tpg1/acls" ls | grep -F "$initiator_iqn" >/dev/null

jq -n \
  --arg target_iqn "$target_iqn" \
  --arg attach_loopdev "$attach_loopdev" \
  --arg initiator_iqn "$initiator_iqn" \
  --argjson attach_lun_id "$attach_lun_id" \
  '{
    version: 1,
    targetLuns: {
      ($target_iqn): {
        operation: "detach",
        provider: "lio",
        source: $attach_loopdev,
        lun: $attach_lun_id,
        client: $initiator_iqn
      }
    },
    apply: {
      allowOffline: true
    }
  }' > "$detach_spec"

mkfs.ext4 -F "$attach_loopdev" >/dev/null
mkdir -p "$tmpdir/lun-mnt"
mount "$attach_loopdev" "$tmpdir/lun-mnt"
lun_mounted=1
printf 'disk-nix target-side LUN sentinel %s %s\n' "$target_iqn" "$attach_loopdev" > "$sentinel_expected"
install -m 0600 "$sentinel_expected" "$tmpdir/lun-mnt/disk-nix-target-lun-sentinel.txt"
cmp "$sentinel_expected" "$tmpdir/lun-mnt/disk-nix-target-lun-sentinel.txt" >/dev/null

fail_tools="$tmpdir/fake-target-lun-detach-tools"
mkdir -p "$fail_tools"
real_targetcli="$(command -v targetcli)"
cat > "$fail_tools/targetcli" <<EOF
#!/usr/bin/env bash
if [[ "\$*" == "/iscsi/$target_iqn/tpg1/acls delete $initiator_iqn" ]]; then
  echo "synthetic target-side LUN detach failure for disk-nix data-survival coverage" >&2
  exit 77
fi
exec "$real_targetcli" "\$@"
EOF
chmod +x "$fail_tools/targetcli"

if PATH="$fail_tools:$PATH" "$disk_nix_bin" apply \
  --spec "$detach_spec" \
  --execute \
  --report-out "$failed_detach_report" \
  --json > "$tmpdir/failed-detach-apply.json"; then
  echo "expected injected target-side LUN detach failure to fail apply" >&2
  exit 1
fi

jq -e \
  --arg target_iqn "$target_iqn" \
  --arg initiator_iqn "$initiator_iqn" \
  --argjson attach_lun_id "$attach_lun_id" \
  '
  .status == "failed"
  and (.executionResults
    | any(
        .argv == ["targetcli", ("/iscsi/" + $target_iqn + "/tpg1/acls"), "delete", $initiator_iqn]
        and .success == false
        and .statusCode == 77
        and (.stderr | contains("synthetic target-side LUN detach failure"))
      ))
  and .partialExecutionRecovery.failedActionId == ("targetluns:" + $target_iqn + ":detach")
  and (.partialExecutionRecovery.retryReviewActionIds | index("targetluns:" + $target_iqn + ":detach") != null)
  and (.recoveryActions | any(.kind == "resume-after-fix"))
  and (.recoveryActions | any(.kind == "domain-recovery"))
' "$tmpdir/failed-detach-apply.json" >/dev/null

cmp "$tmpdir/failed-detach-apply.json" "$failed_detach_report" >/dev/null
cmp "$sentinel_expected" "$tmpdir/lun-mnt/disk-nix-target-lun-sentinel.txt" >/dev/null

if ! "$disk_nix_bin" apply \
  --spec "$detach_spec" \
  --execute \
  --report-out "$detach_report" \
  --json > "$tmpdir/detach-apply.json"; then
  cat "$tmpdir/detach-apply.json" >&2 || true
  cat "$detach_report" >&2 || true
  exit 1
fi

jq -e \
  --arg target_iqn "$target_iqn" \
  --arg initiator_iqn "$initiator_iqn" \
  --argjson attach_lun_id "$attach_lun_id" \
  '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("targetluns:" + $target_iqn + ":detach"))
    | .commands
    | any(.argv == ["targetcli", ("/iscsi/" + $target_iqn + "/tpg1/acls"), "delete", $initiator_iqn])
    and any(.argv == ["targetcli", ("/iscsi/" + $target_iqn + "/tpg1/luns"), "delete", ($attach_lun_id | tostring)]))
  and (.executionResults
    | any(.argv == ["targetcli", ("/iscsi/" + $target_iqn + "/tpg1/luns"), "delete", ($attach_lun_id | tostring)] and .success == true))
' "$tmpdir/detach-apply.json" >/dev/null

cmp "$tmpdir/detach-apply.json" "$detach_report" >/dev/null
cmp "$sentinel_expected" "$tmpdir/lun-mnt/disk-nix-target-lun-sentinel.txt" >/dev/null
if targetcli "/iscsi/$target_iqn/tpg1/luns" ls | grep -E "lun[[:space:]]*$attach_lun_id|lun$attach_lun_id" >/dev/null; then
  echo "target-side LUN $attach_lun_id remained mapped after detach" >&2
  exit 1
fi

jq -n \
  --arg target_iqn "$target_iqn" \
  --arg attach_loopdev "$attach_loopdev" \
  --arg initiator_iqn "$initiator_iqn" \
  --argjson attach_lun_id "$attach_lun_id" \
  '{
    version: 1,
    targetLuns: {
      ($target_iqn): {
        destroy: true,
        provider: "lio",
        source: $attach_loopdev,
        lun: $attach_lun_id,
        client: $initiator_iqn
      }
    },
    apply: {
      allowOffline: true
    }
  }' > "$destroy_refusal_spec"

if "$disk_nix_bin" apply \
  --spec "$destroy_refusal_spec" \
  --json > "$tmpdir/destroy-refusal.json" 2> "$tmpdir/destroy-refusal.stderr"; then
  echo "target-side LUN destroy unexpectedly passed without allowDestructive" >&2
  cat "$tmpdir/destroy-refusal.json" >&2 || true
  exit 1
fi

jq -e \
  --arg target_iqn "$target_iqn" \
  '
  .status == "blocked"
  and .apply.blockedCount == 1
  and .apply.blockedSummary.destructiveCount == 1
  and (.apply.blocked[] | select(.id == ("targetluns:" + $target_iqn + ":destroy") and .operation == "destroy" and .risk == "destructive" and (.reason | contains("allowDestructive=true"))))
  and (.commandPlan | length == 0)
  and (.recoveryActions[] | select(.kind == "review-policy" and (.notes[] | contains("prefer non-destructive alternatives"))))
' "$tmpdir/destroy-refusal.json" >/dev/null

grep -F 'apply policy blocked 1 action(s)' "$tmpdir/destroy-refusal.stderr" >/dev/null

echo "target-side LUN integration smoke test updated lio.writeCache for $target_iqn LUN $lun_id, mapped/unmapped LUN $attach_lun_id, verified failed-and-resumed detach data survival, and verified destructive destroy refusal"
