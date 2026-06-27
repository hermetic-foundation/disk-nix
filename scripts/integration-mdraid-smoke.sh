#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run MD RAID loop-backed integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates,
rescans, replaces a member, degrades, stops, and wipes a temporary loop-backed
MD RAID array. Backing files are created in a temporary directory and removed
during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "MD RAID loop-backed integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" cat cmp grep jq losetup mdadm truncate; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loop_a=""
loop_b=""
loop_c=""
array="/dev/md/disk-nix-md-smoke-$$"

cleanup() {
  if [[ -e "$array" ]]; then
    mdadm --stop "$array" >/dev/null 2>&1 || true
  fi
  for dev in "$loop_a" "$loop_b" "$loop_c"; do
    if [[ -n "$dev" ]]; then
      mdadm --zero-superblock --force "$dev" >/dev/null 2>&1 || true
    fi
  done
  for dev in "$loop_a" "$loop_b" "$loop_c"; do
    if [[ -n "$dev" ]] && losetup --list "$dev" >/dev/null 2>&1; then
      losetup --detach "$dev"
    fi
  done
  rm -rf "$tmpdir"
}
trap cleanup EXIT

backing_a="$tmpdir/disk-nix-md-a.img"
backing_b="$tmpdir/disk-nix-md-b.img"
backing_c="$tmpdir/disk-nix-md-c.img"
spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"
replace_spec="$tmpdir/replace-spec.json"
replace_report="$tmpdir/replace-apply-report.json"
degraded_report="$tmpdir/degraded-apply-report.json"
failed_detach_spec="$tmpdir/failed-detach-spec.json"
failed_detach_report="$tmpdir/failed-detach-report.json"

truncate --size 64M "$backing_a" "$backing_b" "$backing_c"
loop_a="$(losetup --find --show "$backing_a")"
loop_b="$(losetup --find --show "$backing_b")"
loop_c="$(losetup --find --show "$backing_c")"
mdadm --create "$array" --run --metadata=1.2 --level=1 --raid-devices=2 "$loop_a" "$loop_b"

"$disk_nix_bin" inspect "$array" --json > "$tmpdir/inspect.json"
jq -e --arg array "$array" '
  (.matchedNodes // .nodes // [])
  | any(
      .path == $array
      or .id == ("md:" + $array)
      or .id == ("block:" + $array)
      or (.properties // [] | any(.key == "md.path" and .value == $array))
      or (.properties // [] | any(.key == "md.level" and .value == "raid1"))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg array "$array" '{
  version: 1,
  mdRaids: {
    inventory: {
      target: $array,
      operation: "rescan"
    }
  }
}' > "$spec"

"$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"

jq -e --arg array "$array" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "mdraids:inventory:rescan")
    | .commands
    | any(.argv == ["mdadm", "--detail", $array])
    and any(.argv == ["mdadm", "--detail", "--scan"])
    and any(.argv == ["mdadm", "--examine", "--scan"])
    and any(.argv == ["cat", "/proc/mdstat"]))
  and (.executionResults
    | any(.argv == ["mdadm", "--detail", $array] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null
mdadm --detail "$array" >/dev/null

jq -n --arg array "$array" --arg old "$loop_b" --arg new "$loop_c" '{
  version: 1,
  apply: {
    allowOffline: true,
    allowDeviceReplacement: true
  },
  mdRaids: {
    replacement: {
      target: $array,
      replaceDevices: {
        ($old): $new
      }
    }
  }
}' > "$replace_spec"

"$disk_nix_bin" apply \
  --spec "$replace_spec" \
  --execute \
  --report-out "$replace_report" \
  --json > "$tmpdir/replace-apply.json"

jq -e --arg array "$array" --arg old "$loop_b" --arg new "$loop_c" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("mdRaids:replacement:replace-device:" + $old))
    | .commands | any(.argv == ["mdadm", $array, "--replace", $old, "--with", $new]))
  and (.executionResults
    | any(.argv == ["mdadm", $array, "--replace", $old, "--with", $new] and .success == true))
' "$tmpdir/replace-apply.json" >/dev/null

cmp "$tmpdir/replace-apply.json" "$replace_report" >/dev/null
mdadm --wait "$array"
mdadm --detail "$array" > "$tmpdir/replaced-detail.txt"
grep -q "$loop_c" "$tmpdir/replaced-detail.txt"

mdadm "$array" --fail "$loop_c"
mdadm "$array" --remove "$loop_c"
mdadm --examine "$loop_c" > "$tmpdir/stale-member-examine.txt"
grep -Eq 'Array UUID|Raid Level|this +[0-9]+' "$tmpdir/stale-member-examine.txt"
mdadm --detail "$array" > "$tmpdir/degraded-detail.txt"
grep -Eq 'State : .*degraded|Active Devices : 1' "$tmpdir/degraded-detail.txt"

"$disk_nix_bin" inspect "$array" --json > "$tmpdir/degraded-inspect.json"
jq -e --arg array "$array" '
  (.matchedNodes // .nodes // []) as $nodes
  | ($nodes | any(
      .path == $array
      or .id == ("md:" + $array)
      or (.properties // [] | any(.key == "md.path" and .value == $array))
    ))
  and ($nodes | any(
      (.properties // [] | any(.key == "md.degraded-devices" and (.value | tostring) != "0"))
      or (.properties // [] | any(.key == "md.state" and (.value | test("degraded"; "i"))))
    ))
' "$tmpdir/degraded-inspect.json" >/dev/null

"$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$degraded_report" \
  --json > "$tmpdir/degraded-apply.json"

jq -e --arg array "$array" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "mdraids:inventory:rescan")
    | .commands
    | any(.argv == ["mdadm", "--detail", $array])
    and any(.argv == ["cat", "/proc/mdstat"]))
  and (.executionResults
    | any(.argv == ["mdadm", "--detail", $array] and .success == true))
' "$tmpdir/degraded-apply.json" >/dev/null

cmp "$tmpdir/degraded-apply.json" "$degraded_report" >/dev/null

jq -n --arg array "$array" --arg removed "$loop_c" '{
  version: 1,
  apply: {
    allowOffline: true,
    allowPotentialDataLoss: true
  },
  mdRaids: {
    failedDetach: {
      target: $array,
      removeDevices: [$removed]
    }
  }
}' > "$failed_detach_spec"

if "$disk_nix_bin" apply \
  --spec "$failed_detach_spec" \
  --execute \
  --report-out "$failed_detach_report" \
  --json > "$tmpdir/failed-detach-apply.json"; then
  echo "expected failed detach of already-removed MD member to fail apply" >&2
  exit 1
fi

jq -e --arg array "$array" --arg removed "$loop_c" '
  .status == "failed"
  and (.executionResults
    | any(
        (.argv == ["mdadm", $array, "--fail", $removed] or .argv == ["mdadm", $array, "--remove", $removed])
        and .success == false
      ))
  and .partialExecutionRecovery.failedActionId == ("mdRaids:failedDetach:remove-device:" + $removed)
  and (.partialExecutionRecovery.retryReviewActionIds | index("mdRaids:failedDetach:remove-device:" + $removed) != null)
  and (.recoveryActions | any(.kind == "domain-recovery"))
  and (.recoveryActions | any(.kind == "roll-forward-review"))
' "$tmpdir/failed-detach-apply.json" >/dev/null

cmp "$tmpdir/failed-detach-apply.json" "$failed_detach_report" >/dev/null

echo "MD RAID loop-backed integration smoke test rescanned $array from $loop_a and $loop_b, replaced $loop_b with $loop_c, then verified stale-superblock, degraded missing-member rescan, and failed-detach recovery"
