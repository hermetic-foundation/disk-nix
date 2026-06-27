#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run bcache integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates
temporary loop-backed bcache backing and cache devices, mutates a real bcache
sysfs property, stops the generated bcache device, and removes the temporary
backing files during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "bcache integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" blockdev jq losetup make-bcache modprobe truncate; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
backing_loop=""
cache_loop=""
bcachedev=""

cleanup() {
  if [[ -n "$bcachedev" ]] && [[ -e "/sys/block/${bcachedev#/dev/}/bcache/stop" ]]; then
    printf '1\n' > "/sys/block/${bcachedev#/dev/}/bcache/stop" || true
  fi
  if [[ -n "$backing_loop" ]] && losetup --list "$backing_loop" >/dev/null 2>&1; then
    losetup --detach "$backing_loop" || true
  fi
  if [[ -n "$cache_loop" ]] && losetup --list "$cache_loop" >/dev/null 2>&1; then
    losetup --detach "$cache_loop" || true
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

find_bcache_for_backing() {
  local backing_name="$1"
  local node
  for node in /sys/block/bcache*/bcache/backing_dev_name; do
    [[ -e "$node" ]] || continue
    if [[ "$(cat "$node")" == "$backing_name" ]]; then
      printf '/dev/%s\n' "$(basename "$(dirname "$(dirname "$node")")")"
      return 0
    fi
  done
  return 1
}

modprobe bcache

backing="$tmpdir/disk-nix-bcache-backing.img"
cache="$tmpdir/disk-nix-bcache-cache.img"
spec="$tmpdir/property-spec.json"
report="$tmpdir/property-report.json"

truncate --size 256M "$backing"
truncate --size 128M "$cache"
backing_loop="$(losetup --find --show "$backing")"
cache_loop="$(losetup --find --show "$cache")"

make-bcache -B "$backing_loop" -C "$cache_loop" --writeback >/dev/null
printf '%s\n' "$backing_loop" > /sys/fs/bcache/register_quiet || true
printf '%s\n' "$cache_loop" > /sys/fs/bcache/register_quiet || true

backing_name="$(basename "$backing_loop")"
for _ in $(seq 1 50); do
  if bcachedev="$(find_bcache_for_backing "$backing_name")"; then
    break
  fi
  sleep 0.1
done

if [[ -z "$bcachedev" ]]; then
  echo "bcache device did not appear for $backing_loop" >&2
  exit 1
fi

if ! blockdev --getsize64 "$bcachedev" >/dev/null; then
  echo "bcache device $bcachedev is not readable as a block device" >&2
  exit 1
fi

jq -n --arg bcachedev "$bcachedev" '{
  version: 1,
  caches: {
    bcacheSmoke: {
      target: $bcachedev,
      properties: {
        "bcache.cache-mode": "writethrough"
      }
    }
  },
  apply: {
    allowOffline: true,
    allowPropertyChanges: true
  }
}' > "$spec"

if ! "$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/property-apply.json"; then
  cat "$tmpdir/property-apply.json" >&2 || true
  cat "$report" >&2 || true
  exit 1
fi

jq -e --arg bcachedev "$bcachedev" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "caches:bcacheSmoke:set-property:bcache.cache-mode")
    | .commands | any(.argv == ["sh", "-c", "printf '\''%s\\n'\'' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"", "disk-nix-bcache-property", $bcachedev, "writethrough", "cache_mode"]))
  and (.executionResults
    | any(.argv == ["sh", "-c", "printf '\''%s\\n'\'' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"", "disk-nix-bcache-property", $bcachedev, "writethrough", "cache_mode"] and .success == true))
' "$tmpdir/property-apply.json" >/dev/null

cmp "$tmpdir/property-apply.json" "$report" >/dev/null
cache_mode_value="$(cat "/sys/block/${bcachedev#/dev/}/bcache/cache_mode")"
if [[ "$cache_mode_value" != "writethrough" ]] && [[ "$cache_mode_value" != *"[writethrough]"* ]]; then
  echo "bcache cache_mode did not match after disk-nix property mutation" >&2
  exit 1
fi

echo "bcache integration smoke test passed for $bcachedev, including cache_mode property mutation"
