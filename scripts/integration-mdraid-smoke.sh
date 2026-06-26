#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run MD RAID loop-backed integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates,
rescans, stops, and wipes a temporary loop-backed MD RAID array. Backing files
are created in a temporary directory and removed during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "MD RAID loop-backed integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" cat jq losetup mdadm truncate; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loop_a=""
loop_b=""
array="/dev/md/disk-nix-md-smoke-$$"

cleanup() {
  if [[ -e "$array" ]]; then
    mdadm --stop "$array" >/dev/null 2>&1 || true
  fi
  for dev in "$loop_a" "$loop_b"; do
    if [[ -n "$dev" ]]; then
      mdadm --zero-superblock --force "$dev" >/dev/null 2>&1 || true
    fi
  done
  for dev in "$loop_a" "$loop_b"; do
    if [[ -n "$dev" ]] && losetup --list "$dev" >/dev/null 2>&1; then
      losetup --detach "$dev"
    fi
  done
  rm -rf "$tmpdir"
}
trap cleanup EXIT

backing_a="$tmpdir/disk-nix-md-a.img"
backing_b="$tmpdir/disk-nix-md-b.img"
spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"

truncate --size 64M "$backing_a" "$backing_b"
loop_a="$(losetup --find --show "$backing_a")"
loop_b="$(losetup --find --show "$backing_b")"
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

echo "MD RAID loop-backed integration smoke test rescanned $array from $loop_a and $loop_b"
