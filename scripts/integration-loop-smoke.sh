#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run loop-backed integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates
and formats a temporary loop-backed block device. The backing file is created in
a temporary directory and removed during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "loop-backed integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" blockdev jq losetup mkfs.ext4 mount mountpoint resize2fs truncate umount; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loopdev=""
mountpoint="$tmpdir/mnt"

cleanup() {
  if mountpoint -q "$mountpoint"; then
    umount "$mountpoint" || true
  fi
  if [[ -n "$loopdev" ]] && losetup --list "$loopdev" >/dev/null 2>&1; then
    losetup --detach "$loopdev"
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

backing="$tmpdir/disk-nix-loop-smoke.img"
spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"
grow_spec="$tmpdir/grow-spec.json"
grow_report="$tmpdir/grow-report.json"

truncate --size 64M "$backing"
mkdir -p "$mountpoint"
loopdev="$(losetup --find --show "$backing")"
mkfs.ext4 -F -q "$loopdev"
mount "$loopdev" "$mountpoint"

"$disk_nix_bin" inspect "$mountpoint" --json > "$tmpdir/inspect.json"
jq -e --arg loopdev "$loopdev" --arg mountpoint "$mountpoint" '
  (.matchedNodes // .nodes // [])
  | any(
      .path == $loopdev
      or .path == $mountpoint
      or .id == ("block:" + $loopdev)
      or (.properties // [] | any(.key == "mount.target" and .value == $mountpoint))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg loopdev "$loopdev" --arg mountpoint "$mountpoint" '{
  version: 1,
  loopDevices: {
    ($loopdev): {
      operation: "rescan"
    }
  }
}' > "$spec"

if ! "$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"; then
  cat "$tmpdir/apply.json" >&2 || true
  cat "$report" >&2 || true
  exit 1
fi

jq -e '
  .status == "succeeded"
  and (.commandPlan | length) == 1
  and (.executionResults | length) >= 1
  and (.executionResults | all(.success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null

before_size="$(blockdev --getsize64 "$loopdev")"
truncate --size 96M "$backing"
losetup --set-capacity "$loopdev"
after_size="$(blockdev --getsize64 "$loopdev")"
if (( after_size <= before_size )); then
  echo "loop device did not report growth after backing file resize" >&2
  exit 1
fi

jq -n --arg loopdev "$loopdev" --arg mountpoint "$mountpoint" '{
  version: 1,
  filesystems: {
    loopSmoke: {
      device: $loopdev,
      fsType: "ext4",
      mountpoint: $mountpoint,
      resizePolicy: "grow-only"
    }
  }
}' > "$grow_spec"

if ! "$disk_nix_bin" apply \
  --spec "$grow_spec" \
  --execute \
  --report-out "$grow_report" \
  --json > "$tmpdir/grow-apply.json"; then
  cat "$tmpdir/grow-apply.json" >&2 || true
  cat "$grow_report" >&2 || true
  exit 1
fi

jq -e --arg loopdev "$loopdev" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "filesystem:loopSmoke:grow")
    | .commands | any(.argv == ["resize2fs", $loopdev]))
  and (.executionResults | any(.argv == ["resize2fs", $loopdev] and .success == true))
' "$tmpdir/grow-apply.json" >/dev/null

cmp "$tmpdir/grow-apply.json" "$grow_report" >/dev/null

echo "loop-backed integration smoke test passed for $loopdev, including ext4 grow"
