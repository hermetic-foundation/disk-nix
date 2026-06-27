#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run LUKS loop-backed integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates,
formats, opens, closes, and removes a temporary loop-backed LUKS container. The
backing file and temporary key material are created in a temporary directory
and removed during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "LUKS loop-backed integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" cryptsetup grep jq losetup truncate; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loopdev=""
mapper="disk-nix-luks-smoke-$$"

cleanup() {
  if [[ -e "/dev/mapper/$mapper" ]]; then
    cryptsetup close "$mapper" || true
  fi
  if [[ -n "$loopdev" ]] && losetup --list "$loopdev" >/dev/null 2>&1; then
    losetup --detach "$loopdev"
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

backing="$tmpdir/disk-nix-luks-smoke.img"
keyfile="$tmpdir/keyfile"
spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"
property_spec="$tmpdir/property-spec.json"
property_report="$tmpdir/property-report.json"

printf 'disk-nix integration test passphrase\n' > "$keyfile"
chmod 0600 "$keyfile"
truncate --size 64M "$backing"
loopdev="$(losetup --find --show "$backing")"
cryptsetup luksFormat --batch-mode --key-file "$keyfile" "$loopdev"
cryptsetup open --key-file "$keyfile" "$loopdev" "$mapper"

"$disk_nix_bin" inspect "/dev/mapper/$mapper" --json > "$tmpdir/inspect.json"
jq -e --arg mapper "$mapper" '
  (.matchedNodes // .nodes // [])
  | any(
      .path == ("/dev/mapper/" + $mapper)
      or .id == ("block:/dev/mapper/" + $mapper)
      or (.properties // [] | any(.key == "dm.name" and .value == $mapper))
      or (.properties // [] | any(.key == "luks.mapper" and .value == $mapper))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg loopdev "$loopdev" --arg mapper "$mapper" '{
  version: 1,
  apply: {
    allowOffline: true
  },
  luks: {
    devices: {
      luksSmokeLabel: {
        device: $loopdev,
        target: $mapper,
        properties: {
          label: "disknix-luks"
        }
      }
    }
  }
}' > "$property_spec"

"$disk_nix_bin" apply \
  --spec "$property_spec" \
  --execute \
  --report-out "$property_report" \
  --json > "$tmpdir/property-apply.json"

jq -e --arg loopdev "$loopdev" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "luks.devices:luksSmokeLabel:set-property:label")
    | .commands | any(.argv == ["cryptsetup", "config", $loopdev, "--label", "disknix-luks"]))
  and (.executionResults
    | any(.argv == ["cryptsetup", "config", $loopdev, "--label", "disknix-luks"] and .success == true))
' "$tmpdir/property-apply.json" >/dev/null

cmp "$tmpdir/property-apply.json" "$property_report" >/dev/null
cryptsetup luksDump "$loopdev" | grep -Eq 'Label:[[:space:]]+disknix-luks'

jq -n --arg loopdev "$loopdev" --arg mapper "$mapper" '{
  version: 1,
  apply: {
    allowOffline: true
  },
  luks: {
    devices: {
      luksSmoke: {
        device: $loopdev,
        target: $mapper,
        operation: "close"
      }
    }
  }
}' > "$spec"

"$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"

jq -e --arg mapper "$mapper" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "luks.devices:luksSmoke:close")
    | .commands | any(.argv == ["cryptsetup", "close", $mapper]))
  and (.executionResults
    | any(.argv == ["cryptsetup", "close", $mapper] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null
if [[ -e "/dev/mapper/$mapper" ]]; then
  echo "LUKS mapper still exists after disk-nix close operation" >&2
  exit 1
fi

echo "LUKS loop-backed integration smoke test labeled and closed mapper $mapper on $loopdev"
