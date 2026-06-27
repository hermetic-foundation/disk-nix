#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run target-side LUN integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates
a temporary loop-backed LIO block backstore and iSCSI target/LUN mapping,
mutates a target-side LUN property, then removes the temporary target state and
backing file during cleanup.
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

for tool in "$disk_nix_bin" jq losetup modprobe targetcli truncate; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loopdev=""
backstore=""
target_created=0
backstore_created=0

cleanup() {
  if [[ "$target_created" == "1" ]]; then
    targetcli "/iscsi" delete "$target_iqn" >/dev/null 2>&1 || true
  fi
  if [[ "$backstore_created" == "1" ]] && [[ -n "$backstore" ]]; then
    targetcli "/backstores/block" delete "$backstore" >/dev/null 2>&1 || true
  fi
  targetcli saveconfig >/dev/null 2>&1 || true
  if [[ -n "$loopdev" ]] && losetup --list "$loopdev" >/dev/null 2>&1; then
    losetup --detach "$loopdev" || true
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

modprobe target_core_mod
modprobe target_core_iblock || true

backing="$tmpdir/disk-nix-target-lun.img"
property_spec="$tmpdir/property-spec.json"
property_report="$tmpdir/property-report.json"

truncate --size 64M "$backing"
loopdev="$(losetup --find --show "$backing")"
backstore="_${loopdev#/}"
backstore="${backstore//\//_}"

targetcli /backstores/block create "name=$backstore" "dev=$loopdev" >/dev/null
backstore_created=1
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

echo "target-side LUN integration smoke test updated lio.writeCache for $target_iqn LUN $lun_id"
