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

for tool in "$disk_nix_bin" jq losetup mkfs.ext4 truncate; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loopdev=""

cleanup() {
  if [[ -n "$loopdev" ]] && losetup --list "$loopdev" >/dev/null 2>&1; then
    losetup --detach "$loopdev"
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

backing="$tmpdir/disk-nix-loop-smoke.img"
spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"

truncate --size 64M "$backing"
loopdev="$(losetup --find --show "$backing")"
mkfs.ext4 -F -q "$loopdev"

"$disk_nix_bin" inspect "$loopdev" --json > "$tmpdir/inspect.json"
jq -e --arg loopdev "$loopdev" '
  (.matchedNodes // .nodes // [])
  | any(.path == $loopdev or .id == ("block:" + $loopdev))
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg loopdev "$loopdev" '{
  version: 1,
  loopDevices: {
    ($loopdev): {
      operation: "rescan"
    }
  }
}' > "$spec"

"$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"

jq -e '
  .status == "succeeded"
  and (.commandPlan | length) == 1
  and (.executionResults | length) >= 1
  and (.executionResults | all(.success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null

echo "loop-backed integration smoke test passed for $loopdev"
