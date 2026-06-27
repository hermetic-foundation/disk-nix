#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run swap loop-backed integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates,
formats, relabels, and removes a temporary loop-backed swap signature. The
backing file is created in a temporary directory and removed during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "swap loop-backed integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" blkid jq losetup mkswap swaplabel truncate; do
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

backing="$tmpdir/disk-nix-swap-smoke.img"
property_spec="$tmpdir/property-spec.json"
property_report="$tmpdir/property-report.json"

truncate --size 64M "$backing"
loopdev="$(losetup --find --show "$backing")"
mkswap --label disknix-old-swap "$loopdev" >/dev/null

"$disk_nix_bin" inspect "$loopdev" --json > "$tmpdir/inspect.json"
jq -e --arg loopdev "$loopdev" '
  (.matchedNodes // .nodes // [])
  | any(
      .path == $loopdev
      or .id == ("block:" + $loopdev)
      or (.properties // [] | any(.key == "swap.label" and .value == "disknix-old-swap"))
      or (.properties // [] | any(.key == "filesystem.type" and .value == "swap"))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg loopdev "$loopdev" '{
  version: 1,
  apply: {
    allowOffline: true
  },
  swaps: {
    swapSmokeLabel: {
      device: $loopdev,
      properties: {
        label: "disknix-swap"
      }
    }
  }
}' > "$property_spec"

if ! "$disk_nix_bin" apply \
  --spec "$property_spec" \
  --execute \
  --report-out "$property_report" \
  --json > "$tmpdir/property-apply.json"; then
  cat "$tmpdir/property-apply.json" >&2 || true
  cat "$property_report" >&2 || true
  exit 1
fi

jq -e --arg loopdev "$loopdev" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "swaps:swapSmokeLabel:set-property:label")
    | .commands | any(.argv == ["swaplabel", "--label", "disknix-swap", $loopdev]))
  and (.executionResults
    | any(.argv == ["swaplabel", "--label", "disknix-swap", $loopdev] and .success == true))
' "$tmpdir/property-apply.json" >/dev/null

cmp "$tmpdir/property-apply.json" "$property_report" >/dev/null
if [[ "$(blkid -s LABEL -o value "$loopdev")" != "disknix-swap" ]]; then
  echo "loop-backed swap label did not match after disk-nix property mutation" >&2
  exit 1
fi

echo "swap loop-backed integration smoke test labeled $loopdev"
