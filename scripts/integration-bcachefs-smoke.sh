#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run bcachefs loop-backed integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates,
formats, mounts, scrubs, replaces a member, and removes a temporary loop-backed
bcachefs filesystem. The backing files are created in a temporary directory and
removed during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "bcachefs loop-backed integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" bcachefs findmnt jq losetup mount truncate umount; do
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

backing="$tmpdir/disk-nix-bcachefs-smoke.img"
replacement_backing="$tmpdir/disk-nix-bcachefs-replacement.img"
mountpoint="$tmpdir/mnt"
spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"
replace_spec="$tmpdir/replace-spec.json"
replace_report="$tmpdir/replace-report.json"
sentinel_expected="$tmpdir/sentinel.expected"

mkdir -p "$mountpoint"
truncate --size 512M "$backing"
truncate --size 512M "$replacement_backing"
loopdev="$(losetup --find --show "$backing")"
replacement_loopdev="$(losetup --find --show "$replacement_backing")"
bcachefs format --force "$loopdev"
mount -t bcachefs "$loopdev" "$mountpoint"
mounted=1
printf 'disk-nix bcachefs replacement sentinel\n' > "$sentinel_expected"
cp "$sentinel_expected" "$mountpoint/sentinel.txt"

"$disk_nix_bin" inspect "$mountpoint" --json > "$tmpdir/inspect.json"
jq -e --arg mountpoint "$mountpoint" '
  (.matchedNodes // .nodes // [])
  | any(
      .path == $mountpoint
      or .id == ("filesystem:" + $mountpoint)
      or (.properties // [] | any(.key == "bcachefs.mount-target" and .value == $mountpoint))
      or (.properties // [] | any(.key == "filesystem.type" and .value == "bcachefs"))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg loopdev "$loopdev" --arg mountpoint "$mountpoint" '{
  version: 1,
  filesystems: {
    bcachefsSmoke: {
      device: $loopdev,
      fsType: "bcachefs",
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
  and (.commandPlan[] | select(.actionId == "filesystem:bcachefsSmoke:scrub")
    | .commands | any(.argv == ["bcachefs", "scrub", $mountpoint]))
  and (.executionResults
    | any(.argv == ["bcachefs", "scrub", $mountpoint] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null
findmnt --target "$mountpoint" --types bcachefs >/dev/null

jq -n --arg loopdev "$loopdev" --arg replacement_loopdev "$replacement_loopdev" --arg mountpoint "$mountpoint" '{
  version: 1,
  filesystems: {
    bcachefsReplacement: {
      device: $loopdev,
      fsType: "bcachefs",
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
  and (.commandPlan[] | select(.actionId == ("filesystems:bcachefsReplacement:replace-device:" + $old))
    | (.commands | any(.argv == ["bcachefs", "fs", "usage", $mountpoint]))
    and (.commands | any(.argv == ["bcachefs", "device", "add", $mountpoint, $new]))
    and (.commands | any(.argv == ["bcachefs", "data", "rereplicate", $mountpoint]))
    and (.commands | any(.argv == ["bcachefs", "device", "remove", $mountpoint, $old])))
  and (.executionResults | any(.argv == ["bcachefs", "device", "add", $mountpoint, $new] and .success == true))
  and (.executionResults | any(.argv == ["bcachefs", "data", "rereplicate", $mountpoint] and .success == true))
  and (.executionResults | any(.argv == ["bcachefs", "device", "remove", $mountpoint, $old] and .success == true))
' "$tmpdir/replace-apply.json" >/dev/null

cmp "$tmpdir/replace-apply.json" "$replace_report" >/dev/null
findmnt --target "$mountpoint" --types bcachefs >/dev/null
cmp "$sentinel_expected" "$mountpoint/sentinel.txt" >/dev/null
bcachefs fs usage "$mountpoint" >/dev/null
bcachefs show-super "$replacement_loopdev" >/dev/null

echo "bcachefs loop-backed integration smoke test scrubbed and replaced $loopdev with $replacement_loopdev mounted at $mountpoint"
