#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run Btrfs loop-backed integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates,
formats, mounts, scrubs, and removes a temporary loop-backed Btrfs filesystem.
The backing file is created in a temporary directory and removed during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "Btrfs loop-backed integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" btrfs findmnt jq losetup mkfs.btrfs mount truncate umount; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loopdev=""
mounted=0

cleanup() {
  if [[ "$mounted" == "1" ]]; then
    umount "$tmpdir/mnt" || true
  fi
  if [[ -n "$loopdev" ]] && losetup --list "$loopdev" >/dev/null 2>&1; then
    losetup --detach "$loopdev"
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

backing="$tmpdir/disk-nix-btrfs-smoke.img"
mountpoint="$tmpdir/mnt"
spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"

mkdir -p "$mountpoint"
truncate --size 128M "$backing"
loopdev="$(losetup --find --show "$backing")"
mkfs.btrfs --force --quiet "$loopdev"
mount -t btrfs "$loopdev" "$mountpoint"
mounted=1

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
  and (.commandPlan[] | select(.actionId == "filesystem:btrfsSmoke:scrub")
    | .commands | any(.argv == ["btrfs", "scrub", "start", "-B", $mountpoint]))
  and (.executionResults
    | any(.argv == ["btrfs", "scrub", "start", "-B", $mountpoint] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null
findmnt --target "$mountpoint" --types btrfs >/dev/null

echo "Btrfs loop-backed integration smoke test passed for $loopdev mounted at $mountpoint"
