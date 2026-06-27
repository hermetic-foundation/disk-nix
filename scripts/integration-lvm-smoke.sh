#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run LVM loop-backed integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates,
rescans, removes, and wipes a temporary loop-backed LVM physical volume, volume
group, logical volume, thin pool, thin volume, and snapshot. The backing file is
created in a temporary directory and removed during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "LVM loop-backed integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" jq losetup lvchange lvconvert lvcreate lvs pvcreate pvremove pvscan pvs tr truncate vgchange vgcreate vgremove vgscan vgs; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loopdev=""
vg="disk_nix_lvm_smoke_$$"

cleanup() {
  if vgs "$vg" >/dev/null 2>&1; then
    vgchange --activate n "$vg" >/dev/null 2>&1 || true
    vgremove --force --force --yes "$vg" >/dev/null 2>&1 || true
  fi
  if [[ -n "$loopdev" ]]; then
    pvremove --force --force --yes "$loopdev" >/dev/null 2>&1 || true
  fi
  if [[ -n "$loopdev" ]] && losetup --list "$loopdev" >/dev/null 2>&1; then
    losetup --detach "$loopdev"
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

backing="$tmpdir/disk-nix-lvm-smoke.img"
spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"
property_spec="$tmpdir/property-spec.json"
property_report="$tmpdir/property-report.json"

truncate --size 512M "$backing"
loopdev="$(losetup --find --show "$backing")"
pvcreate --force --yes "$loopdev"
vgcreate "$vg" "$loopdev"
lvcreate --yes --size 64M --name origin "$vg"
lvcreate --yes --type thin-pool --size 128M --name thinpool "$vg"
lvcreate --yes --virtualsize 64M --thinpool "$vg/thinpool" --name thinvol "$vg"
lvcreate --yes --snapshot --size 32M --name origin_snap "$vg/origin"
lvcreate --yes --type cache-pool --size 64M --name cachepool "$vg"
lvconvert --yes --type cache --cachepool "$vg/cachepool" "$vg/origin"

"$disk_nix_bin" inspect "$vg" --json > "$tmpdir/inspect.json"
jq -e --arg vg "$vg" '
  (.matchedNodes // .nodes // [])
  | any(
      .name == $vg
      or .id == ("lvm-vg:" + $vg)
      or (.properties // [] | any(.key == "lvm.vg-name" and .value == $vg))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg origin "$vg/origin" '{
  version: 1,
  apply: {
    allowOffline: true
  },
  lvmCaches: {
    ($origin): {
      properties: {
        "lvm.cache-mode": "writethrough"
      }
    }
  }
}' > "$property_spec"

"$disk_nix_bin" apply \
  --spec "$property_spec" \
  --execute \
  --report-out "$property_report" \
  --json > "$tmpdir/property-apply.json"

jq -e --arg origin "$vg/origin" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("lvmCaches:" + $origin + ":set-property:lvm.cache-mode"))
    | .commands | any(.argv == ["lvchange", "--cachemode", "writethrough", $origin]))
  and (.executionResults
    | any(.argv == ["lvchange", "--cachemode", "writethrough", $origin] and .success == true))
' "$tmpdir/property-apply.json" >/dev/null

cmp "$tmpdir/property-apply.json" "$property_report" >/dev/null
if [[ "$(lvs --noheadings -o cache_mode "$vg/origin" | tr -d '[:space:]')" != "writethrough" ]]; then
  echo "LVM cache mode did not match after disk-nix property mutation" >&2
  exit 1
fi

jq -n \
  --arg vg "$vg" \
  --arg origin "$vg/origin" \
  --arg thinpool "$vg/thinpool" \
  --arg snapshot "$vg/origin_snap" \
  '{
  version: 1,
  volumeGroups: {
    ($vg): {
      operation: "rescan"
    }
  },
  volumes: {
    ($origin): {
      operation: "rescan"
    }
  },
  thinPools: {
    ($thinpool): {
      operation: "rescan"
    }
  },
  lvmSnapshots: {
    ($snapshot): {
      operation: "rescan"
    }
  }
}' > "$spec"

"$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"

jq -e \
  --arg vg "$vg" \
  --arg origin "$vg/origin" \
  --arg thinpool "$vg/thinpool" \
  --arg snapshot "$vg/origin_snap" \
  '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("volumegroups:" + $vg + ":rescan"))
    | .commands
    | any(.argv == ["pvscan", "--cache"])
    and any(.argv == ["vgscan"])
    and any(.argv == ["vgchange", "--refresh", $vg]))
  and (.commandPlan[] | select(.actionId == ("volumes:" + $origin + ":rescan"))
    | .commands
    | any(.argv == ["lvs", "--reportformat", "json", $origin])
    and any(.argv == ["disk-nix", "inspect", $origin]))
  and (.commandPlan[] | select(.actionId == ("thinpools:" + $thinpool + ":rescan"))
    | .commands
    | any(.argv == ["lvs", "--reportformat", "json", "-o", "lv_name,lv_size,data_percent,metadata_percent,seg_monitor", $thinpool])
    and any(.argv == ["disk-nix", "inspect", $thinpool]))
  and (.commandPlan[] | select(.actionId == ("lvmsnapshots:" + $snapshot + ":rescan"))
    | .commands
    | any(.argv == ["lvs", "--reportformat", "json", "-o", "lv_name,origin,lv_attr,data_percent,metadata_percent,lv_size", $snapshot]))
  and (.executionResults
    | any(.argv == ["vgchange", "--refresh", $vg] and .success == true)
    and any(.argv == ["lvs", "--reportformat", "json", $origin] and .success == true)
    and any(.argv == ["lvs", "--reportformat", "json", "-o", "lv_name,lv_size,data_percent,metadata_percent,seg_monitor", $thinpool] and .success == true)
    and any(.argv == ["lvs", "--reportformat", "json", "-o", "lv_name,origin,lv_attr,data_percent,metadata_percent,lv_size", $snapshot] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null
vgs --reportformat json "$vg" >/dev/null
pvs --reportformat json "$loopdev" >/dev/null
lvs --reportformat json "$vg/origin" >/dev/null
lvs --reportformat json "$vg/thinpool" >/dev/null
lvs --reportformat json "$vg/thinvol" >/dev/null
lvs --reportformat json "$vg/origin_snap" >/dev/null

echo "LVM loop-backed integration smoke test refreshed $vg with cached origin, thin pool, thin volume, and snapshot on $loopdev"
