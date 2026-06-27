#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run bcache integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates
temporary loop-backed bcache backing and cache devices, mutates a real bcache
sysfs property, detaches and reattaches the real cache set, replaces the cache
device, verifies cache-device failure-state recovery, stops the generated
bcache device, and removes the temporary backing files during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "bcache integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" blockdev cat cmp jq losetup make-bcache modprobe readlink truncate; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
backing_loop=""
cache_loop=""
replacement_cache_loop=""
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
  if [[ -n "$replacement_cache_loop" ]] && losetup --list "$replacement_cache_loop" >/dev/null 2>&1; then
    losetup --detach "$replacement_cache_loop" || true
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
replacement_cache="$tmpdir/disk-nix-bcache-replacement-cache.img"
spec="$tmpdir/property-spec.json"
report="$tmpdir/property-report.json"
detach_spec="$tmpdir/detach-spec.json"
detach_report="$tmpdir/detach-report.json"
failed_attach_spec="$tmpdir/failed-attach-spec.json"
failed_attach_report="$tmpdir/failed-attach-report.json"
attach_spec="$tmpdir/attach-spec.json"
attach_report="$tmpdir/attach-report.json"
replace_spec="$tmpdir/replace-spec.json"
replace_report="$tmpdir/replace-report.json"
rescan_spec="$tmpdir/rescan-spec.json"
rescan_report="$tmpdir/rescan-report.json"

truncate --size 256M "$backing"
truncate --size 128M "$cache"
truncate --size 128M "$replacement_cache"
backing_loop="$(losetup --find --show "$backing")"
cache_loop="$(losetup --find --show "$cache")"
replacement_cache_loop="$(losetup --find --show "$replacement_cache")"

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

cache_set_uuid="$(basename "$(readlink -f "/sys/block/${bcachedev#/dev/}/bcache/cache")")"
if [[ -z "$cache_set_uuid" ]] || [[ "$cache_set_uuid" == "cache" ]]; then
  echo "could not determine bcache cache-set UUID for $bcachedev" >&2
  exit 1
fi

jq -n --arg bcachedev "$bcachedev" --arg cache_set_uuid "$cache_set_uuid" '{
  version: 1,
  caches: {
    bcacheSmoke: {
      target: $bcachedev,
      removeDevices: [$cache_set_uuid]
    }
  },
  apply: {
    allowOffline: true,
    allowPotentialDataLoss: true
  }
}' > "$detach_spec"

"$disk_nix_bin" apply \
  --spec "$detach_spec" \
  --execute \
  --report-out "$detach_report" \
  --json > "$tmpdir/detach-apply.json"

jq -e --arg bcachedev "$bcachedev" --arg cache_set_uuid "$cache_set_uuid" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("caches:bcacheSmoke:remove-device:" + $cache_set_uuid))
    | .commands | any(.argv == ["sh", "-c", "printf '\''1\\n'\'' > \"/sys/block/${1#/dev/}/bcache/detach\"", "disk-nix-bcache-detach", $bcachedev]))
  and (.executionResults
    | any(.argv == ["sh", "-c", "printf '\''1\\n'\'' > \"/sys/block/${1#/dev/}/bcache/detach\"", "disk-nix-bcache-detach", $bcachedev] and .success == true))
' "$tmpdir/detach-apply.json" >/dev/null

cmp "$tmpdir/detach-apply.json" "$detach_report" >/dev/null
cat "/sys/block/${bcachedev#/dev/}/bcache/state" >/dev/null

invalid_cache_set_uuid="00000000-0000-0000-0000-000000000000"
jq -n --arg bcachedev "$bcachedev" --arg invalid_cache_set_uuid "$invalid_cache_set_uuid" '{
  version: 1,
  caches: {
    bcacheFailedAttach: {
      target: $bcachedev,
      addDevices: [$invalid_cache_set_uuid]
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$failed_attach_spec"

if "$disk_nix_bin" apply \
  --spec "$failed_attach_spec" \
  --execute \
  --report-out "$failed_attach_report" \
  --json > "$tmpdir/failed-attach-apply.json"; then
  echo "expected failed bcache cache-set attach to fail apply" >&2
  exit 1
fi

jq -e --arg bcachedev "$bcachedev" --arg invalid_cache_set_uuid "$invalid_cache_set_uuid" '
  .status == "failed"
  and (.executionResults
    | any(.argv == ["sh", "-c", "printf '\''%s\\n'\'' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"", "disk-nix-bcache-attach", $bcachedev, $invalid_cache_set_uuid] and .success == false))
  and .partialExecutionRecovery.failedActionId == ("caches:bcacheFailedAttach:add-device:" + $invalid_cache_set_uuid)
  and (.partialExecutionRecovery.retryReviewActionIds | index("caches:bcacheFailedAttach:add-device:" + $invalid_cache_set_uuid) != null)
  and (.recoveryActions | any(.kind == "domain-recovery"))
  and (.recoveryActions | any(.kind == "roll-forward-review"))
' "$tmpdir/failed-attach-apply.json" >/dev/null

cmp "$tmpdir/failed-attach-apply.json" "$failed_attach_report" >/dev/null
cat "/sys/block/${bcachedev#/dev/}/bcache/state" >/dev/null

jq -n --arg bcachedev "$bcachedev" --arg cache_set_uuid "$cache_set_uuid" '{
  version: 1,
  caches: {
    bcacheSmoke: {
      target: $bcachedev,
      addDevices: [$cache_set_uuid],
      properties: {
        "bcache.cache-mode": "writethrough"
      }
    }
  },
  apply: {
    allowOffline: true,
    allowPropertyChanges: true
  }
}' > "$attach_spec"

"$disk_nix_bin" apply \
  --spec "$attach_spec" \
  --execute \
  --report-out "$attach_report" \
  --json > "$tmpdir/attach-apply.json"

jq -e --arg bcachedev "$bcachedev" --arg cache_set_uuid "$cache_set_uuid" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("caches:bcacheSmoke:add-device:" + $cache_set_uuid))
    | .commands | any(.argv == ["sh", "-c", "printf '\''%s\\n'\'' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"", "disk-nix-bcache-attach", $bcachedev, $cache_set_uuid]))
  and (.commandPlan[] | select(.actionId == "caches:bcacheSmoke:set-property:bcache.cache-mode")
    | .commands | any(.argv == ["sh", "-c", "printf '\''%s\\n'\'' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"", "disk-nix-bcache-property", $bcachedev, "writethrough", "cache_mode"]))
  and (.executionResults
    | any(.argv == ["sh", "-c", "printf '\''%s\\n'\'' \"$2\" > \"/sys/block/${1#/dev/}/bcache/attach\"", "disk-nix-bcache-attach", $bcachedev, $cache_set_uuid] and .success == true)
    and any(.argv == ["sh", "-c", "printf '\''%s\\n'\'' \"$2\" > \"/sys/block/${1#/dev/}/bcache/$3\"", "disk-nix-bcache-property", $bcachedev, "writethrough", "cache_mode"] and .success == true))
' "$tmpdir/attach-apply.json" >/dev/null

cmp "$tmpdir/attach-apply.json" "$attach_report" >/dev/null
cache_mode_value="$(cat "/sys/block/${bcachedev#/dev/}/bcache/cache_mode")"
if [[ "$cache_mode_value" != "writethrough" ]] && [[ "$cache_mode_value" != *"[writethrough]"* ]]; then
  echo "bcache cache_mode did not match after disk-nix cache reattach" >&2
  exit 1
fi

jq -n --arg bcachedev "$bcachedev" --arg old_cache "$cache_loop" --arg new_cache "$replacement_cache_loop" --arg cache_set_uuid "$cache_set_uuid" '{
  version: 1,
  caches: {
    bcacheReplacement: {
      target: $bcachedev,
      replaceDevices: {
        ($old_cache): $new_cache
      },
      cacheSetUuid: $cache_set_uuid
    }
  },
  apply: {
    allowOffline: true,
    allowDeviceReplacement: true
  }
}' > "$replace_spec"

"$disk_nix_bin" apply \
  --spec "$replace_spec" \
  --execute \
  --report-out "$replace_report" \
  --json > "$tmpdir/replace-apply.json"

jq -e --arg bcachedev "$bcachedev" --arg old_cache "$cache_loop" --arg new_cache "$replacement_cache_loop" --arg cache_set_uuid "$cache_set_uuid" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("caches:bcacheReplacement:replace-device:" + $old_cache))
    | .commands | any(.argv == [
        "sh",
        "-c",
        "make-bcache -C \"$2\" --cset-uuid \"$3\" --writeback && printf '\''1\\n'\'' > \"/sys/block/${1#/dev/}/bcache/detach\" && printf '\''%s\\n'\'' \"$3\" > \"/sys/block/${1#/dev/}/bcache/attach\"",
        "disk-nix-bcache-replace",
        $bcachedev,
        $new_cache,
        $cache_set_uuid
      ]))
  and (.executionResults
    | any(.argv == [
        "sh",
        "-c",
        "make-bcache -C \"$2\" --cset-uuid \"$3\" --writeback && printf '\''1\\n'\'' > \"/sys/block/${1#/dev/}/bcache/detach\" && printf '\''%s\\n'\'' \"$3\" > \"/sys/block/${1#/dev/}/bcache/attach\"",
        "disk-nix-bcache-replace",
        $bcachedev,
        $new_cache,
        $cache_set_uuid
      ] and .success == true))
' "$tmpdir/replace-apply.json" >/dev/null

cmp "$tmpdir/replace-apply.json" "$replace_report" >/dev/null
blockdev --getsize64 "$bcachedev" >/dev/null
cache_mode_value="$(cat "/sys/block/${bcachedev#/dev/}/bcache/cache_mode")"
if [[ "$cache_mode_value" != "writethrough" ]] && [[ "$cache_mode_value" != *"[writethrough]"* ]] && [[ "$cache_mode_value" != "writeback" ]] && [[ "$cache_mode_value" != *"[writeback]"* ]]; then
  echo "bcache cache_mode was not readable after disk-nix cache replacement" >&2
  exit 1
fi

jq -n --arg bcachedev "$bcachedev" '{
  version: 1,
  caches: {
    bcacheSmoke: {
      target: $bcachedev,
      operation: "rescan"
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$rescan_spec"

if ! "$disk_nix_bin" apply \
  --spec "$rescan_spec" \
  --execute \
  --report-out "$rescan_report" \
  --json > "$tmpdir/rescan-apply.json"; then
  cat "$tmpdir/rescan-apply.json" >&2 || true
  cat "$rescan_report" >&2 || true
  exit 1
fi

jq -e --arg bcachedev "$bcachedev" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "caches:bcacheSmoke:rescan")
    | .commands | any(.argv == ["disk-nix", "inspect", $bcachedev]))
  and (.executionResults
    | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", $bcachedev, "state"] and .success == true))
  and (.executionResults
    | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", $bcachedev, "cache_mode"] and .success == true))
  and (.executionResults
    | any(.argv == ["sh", "-c", "cat \"/sys/block/${1#/dev/}/bcache/$2\"", "disk-nix-bcache-read", $bcachedev, "dirty_data"] and .success == true))
' "$tmpdir/rescan-apply.json" >/dev/null

cmp "$tmpdir/rescan-apply.json" "$rescan_report" >/dev/null

echo "bcache integration smoke test passed for $bcachedev, including cache_mode property mutation, cache detach/reattach, failed cache attach recovery, cache replacement, and read-only rescan"
