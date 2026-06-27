#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run Btrfs loop-backed integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates,
formats, mounts, scrubs, replaces a member, and removes a temporary loop-backed
Btrfs filesystem. The backing files are created in a temporary directory and
removed during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "Btrfs loop-backed integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" btrfs findmnt grep jq losetup mkfs.btrfs mount truncate umount; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loopdev=""
replacement_loopdev=""
mounted=0

cleanup() {
  if [[ "$mounted" == "1" ]]; then
    umount "$tmpdir/mnt" || true
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

backing="$tmpdir/disk-nix-btrfs-smoke.img"
replacement_backing="$tmpdir/disk-nix-btrfs-replacement.img"
mountpoint="$tmpdir/mnt"
spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"
property_spec="$tmpdir/property-spec.json"
property_report="$tmpdir/property-report.json"
replace_spec="$tmpdir/replace-spec.json"
replace_report="$tmpdir/replace-report.json"
sentinel_expected="$tmpdir/sentinel.expected"

mkdir -p "$mountpoint"
truncate --size 128M "$backing"
truncate --size 128M "$replacement_backing"
loopdev="$(losetup --find --show "$backing")"
replacement_loopdev="$(losetup --find --show "$replacement_backing")"
mkfs.btrfs --force --quiet "$loopdev"
mount -t btrfs "$loopdev" "$mountpoint"
mounted=1
printf 'disk-nix Btrfs replacement sentinel\n' > "$sentinel_expected"
cp "$sentinel_expected" "$mountpoint/sentinel.txt"

"$disk_nix_bin" inspect "$mountpoint" --json > "$tmpdir/inspect.json"
jq -e --arg mountpoint "$mountpoint" '
  (.matchedNodes // .nodes // [])
  | any(
      .path == $mountpoint
      or .id == ("filesystem:" + $mountpoint)
      or (.properties // [] | any(.key == "btrfs.mount-target" and .value == $mountpoint))
      or (.properties // [] | any(.key == "filesystem.type" and .value == "btrfs"))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg loopdev "$loopdev" --arg mountpoint "$mountpoint" '{
  version: 1,
  filesystems: {
    btrfsSmokeLabel: {
      device: $loopdev,
      fsType: "btrfs",
      mountpoint: $mountpoint,
      properties: {
        label: "disknix-btrfs"
      }
    }
  }
}' > "$property_spec"

"$disk_nix_bin" apply \
  --spec "$property_spec" \
  --execute \
  --report-out "$property_report" \
  --json > "$tmpdir/property-apply.json"

jq -e --arg mountpoint "$mountpoint" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "filesystems:btrfsSmokeLabel:set-property:label")
    | .commands | any(.argv == ["btrfs", "filesystem", "label", $mountpoint, "disknix-btrfs"]))
  and (.executionResults
    | any(.argv == ["btrfs", "filesystem", "label", $mountpoint, "disknix-btrfs"] and .success == true))
' "$tmpdir/property-apply.json" >/dev/null

cmp "$tmpdir/property-apply.json" "$property_report" >/dev/null
btrfs filesystem label "$mountpoint" | grep -qx 'disknix-btrfs'

jq -n --arg loopdev "$loopdev" --arg mountpoint "$mountpoint" '{
  version: 1,
  filesystems: {
    btrfsSmoke: {
      device: $loopdev,
      fsType: "btrfs",
      mountpoint: $mountpoint,
      operation: "scrub"
    }
  }
}' > "$spec"

"$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"

jq -e --arg mountpoint "$mountpoint" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "filesystems:btrfsSmoke:scrub")
    | .commands | any(.argv == ["btrfs", "scrub", "start", "-B", $mountpoint]))
  and (.executionResults
    | any(.argv == ["btrfs", "scrub", "start", "-B", $mountpoint] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null
findmnt --target "$mountpoint" --types btrfs >/dev/null

jq -n --arg loopdev "$loopdev" --arg replacement_loopdev "$replacement_loopdev" --arg mountpoint "$mountpoint" '{
  version: 1,
  filesystems: {
    btrfsReplacement: {
      device: $loopdev,
      fsType: "btrfs",
      mountpoint: $mountpoint,
      replaceDevices: {
        ($loopdev): $replacement_loopdev
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

jq -e --arg mountpoint "$mountpoint" --arg old "$loopdev" --arg new "$replacement_loopdev" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("filesystems:btrfsReplacement:replace-device:" + $old))
    | .commands | any(.argv == ["btrfs", "replace", "start", $old, $new, $mountpoint]))
  and (.executionResults
    | any(.argv == ["btrfs", "replace", "start", $old, $new, $mountpoint] and .success == true))
' "$tmpdir/replace-apply.json" >/dev/null

cmp "$tmpdir/replace-apply.json" "$replace_report" >/dev/null
findmnt --target "$mountpoint" --types btrfs >/dev/null
cmp "$sentinel_expected" "$mountpoint/sentinel.txt" >/dev/null
replacement_visible=0
for _ in {1..60}; do
  if btrfs filesystem show "$mountpoint" | grep -F "$replacement_loopdev" >/dev/null; then
    replacement_visible=1
    break
  fi
  btrfs replace status "$mountpoint" >/dev/null || true
  sleep 1
done
if [[ "$replacement_visible" != "1" ]]; then
  echo "Btrfs replacement device did not appear in filesystem inventory: $replacement_loopdev" >&2
  btrfs replace status "$mountpoint" >&2 || true
  btrfs filesystem show "$mountpoint" >&2 || true
  exit 1
fi

echo "Btrfs loop-backed integration smoke test labeled, scrubbed, and replaced $loopdev with $replacement_loopdev mounted at $mountpoint"
