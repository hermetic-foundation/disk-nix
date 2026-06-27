#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run ZFS loop-backed integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates,
scrubs, replaces a pool member, destroys, and removes a temporary loop-backed
ZFS pool. The backing files are created in a temporary directory and removed
during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "ZFS loop-backed integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" jq losetup mountpoint truncate zfs zpool; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loopdev=""
replacement_loopdev=""
pool="disknix_zfs_smoke_$$"
pool_created=0

cleanup() {
  if [[ "$pool_created" == "1" ]]; then
    zpool destroy "$pool" >/dev/null 2>&1 || true
  fi
  if [[ -n "$loopdev" ]] && losetup --list "$loopdev" >/dev/null 2>&1; then
    losetup --detach "$loopdev"
  fi
  if [[ -n "$replacement_loopdev" ]] && losetup --list "$replacement_loopdev" >/dev/null 2>&1; then
    losetup --detach "$replacement_loopdev"
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

backing="$tmpdir/disk-nix-zfs-smoke.img"
replacement_backing="$tmpdir/disk-nix-zfs-replacement.img"
mountpoint_path="$tmpdir/mnt"
spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"
property_spec="$tmpdir/property-spec.json"
property_report="$tmpdir/property-report.json"
replace_spec="$tmpdir/replace-spec.json"
replace_report="$tmpdir/replace-report.json"

mkdir -p "$mountpoint_path"
truncate --size 512M "$backing"
truncate --size 512M "$replacement_backing"
loopdev="$(losetup --find --show "$backing")"
replacement_loopdev="$(losetup --find --show "$replacement_backing")"
zpool create -f -m "$mountpoint_path" "$pool" "$loopdev"
pool_created=1

"$disk_nix_bin" inspect "$pool" --json > "$tmpdir/inspect.json"
jq -e --arg pool "$pool" --arg mountpoint_path "$mountpoint_path" '
  (.matchedNodes // .nodes // [])
  | any(
      .name == $pool
      or .id == ("zfs-pool:" + $pool)
      or .path == $mountpoint_path
      or (.properties // [] | any(.key == "zfs.health" and .value == "ONLINE"))
      or (.properties // [] | any(.key == "zfs.mountpoint" and .value == $mountpoint_path))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg pool "$pool" '{
  version: 1,
  pools: {
    ($pool): {
      properties: {
        autotrim: "on"
      }
    }
  }
}' > "$property_spec"

"$disk_nix_bin" apply \
  --spec "$property_spec" \
  --execute \
  --report-out "$property_report" \
  --json > "$tmpdir/property-apply.json"

jq -e --arg pool "$pool" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("pools:" + $pool + ":set-property:autotrim"))
    | .commands | any(.argv == ["zpool", "set", "autotrim=on", $pool]))
  and (.executionResults
    | any(.argv == ["zpool", "set", "autotrim=on", $pool] and .success == true))
' "$tmpdir/property-apply.json" >/dev/null

cmp "$tmpdir/property-apply.json" "$property_report" >/dev/null
if [[ "$(zpool get -H -o value autotrim "$pool")" != "on" ]]; then
  echo "ZFS pool autotrim property did not match after disk-nix mutation" >&2
  exit 1
fi

jq -n --arg pool "$pool" '{
  version: 1,
  pools: {
    ($pool): {
      operation: "scrub"
    }
  }
}' > "$spec"

"$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"

jq -e --arg pool "$pool" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("pools:" + $pool + ":scrub"))
    | .commands | any(.argv == ["zpool", "scrub", $pool]))
  and (.executionResults
    | any(.argv == ["zpool", "scrub", $pool] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null
zpool status "$pool" >/dev/null
mountpoint -q "$mountpoint_path"

jq -n --arg pool "$pool" --arg old "$loopdev" --arg new "$replacement_loopdev" '{
  version: 1,
  pools: {
    ($pool): {
      replaceDevices: {
        ($old): $new
      }
    }
  },
  apply: {
    allowDeviceReplacement: true
  }
}' > "$replace_spec"

"$disk_nix_bin" apply \
  --spec "$replace_spec" \
  --execute \
  --report-out "$replace_report" \
  --json > "$tmpdir/replace-apply.json"

jq -e --arg pool "$pool" --arg old "$loopdev" --arg new "$replacement_loopdev" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("pools:" + $pool + ":replace-device:" + $old))
    | .commands | any(.argv == ["zpool", "replace", $pool, $old, $new]))
  and (.executionResults
    | any(.argv == ["zpool", "replace", $pool, $old, $new] and .success == true))
' "$tmpdir/replace-apply.json" >/dev/null

cmp "$tmpdir/replace-apply.json" "$replace_report" >/dev/null
zpool status "$pool" >/dev/null
zpool status -P "$pool" | grep -F "$replacement_loopdev" >/dev/null
mountpoint -q "$mountpoint_path"

echo "ZFS loop-backed integration smoke test set autotrim, scrubbed, and replaced $loopdev with $replacement_loopdev in $pool"
