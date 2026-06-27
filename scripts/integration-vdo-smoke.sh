#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run VDO integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test inspects
real VDO management state and changes the write policy for the disposable
volume provided through DISK_NIX_VDO_NAME.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "VDO integration smoke test must run as root" >&2
  exit 2
fi

vdo_name="${DISK_NIX_VDO_NAME:-}"
if [[ -z "$vdo_name" ]]; then
  cat >&2 <<'MSG'
DISK_NIX_VDO_NAME is required.

Example:
  DISK_NIX_VDO_NAME=archive
MSG
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"
write_policy="${DISK_NIX_VDO_WRITE_POLICY:-sync}"
case "$write_policy" in
  auto | sync | async) ;;
  *)
    echo "DISK_NIX_VDO_WRITE_POLICY must be one of: auto, sync, async" >&2
    exit 2
    ;;
esac

for tool in "$disk_nix_bin" grep jq vdo vdostats; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT

spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"
property_spec="$tmpdir/property-spec.json"
property_report="$tmpdir/property-report.json"

vdo status --name "$vdo_name" > "$tmpdir/vdo-status.txt"
vdostats --human-readable "$vdo_name" > "$tmpdir/vdostats.txt"

"$disk_nix_bin" inspect "$vdo_name" --json > "$tmpdir/inspect.json"
jq -e --arg vdo_name "$vdo_name" '
  (.matchedNodes // .nodes // [])
  | any(
      .name == $vdo_name
      or .id == ("vdo:" + $vdo_name)
      or .path == ("/dev/mapper/" + $vdo_name)
      or (.properties // [] | any(.key == "vdo.storage-device"))
      or (.properties // [] | any(.key == "vdo.operating-mode"))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg vdo_name "$vdo_name" --arg write_policy "$write_policy" '{
  version: 1,
  apply: {
    allowOffline: true
  },
  vdoVolumes: {
    ($vdo_name): {
      properties: {
        writePolicy: $write_policy
      }
    }
  }
}' > "$property_spec"

"$disk_nix_bin" apply \
  --spec "$property_spec" \
  --execute \
  --report-out "$property_report" \
  --json > "$tmpdir/property-apply.json"

jq -e --arg vdo_name "$vdo_name" --arg write_policy "$write_policy" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("vdoVolumes:" + $vdo_name + ":set-property:writePolicy"))
    | .commands | any(.argv == ["vdo", "changeWritePolicy", "--name", $vdo_name, "--writePolicy", $write_policy]))
  and (.executionResults
    | any(.argv == ["vdo", "changeWritePolicy", "--name", $vdo_name, "--writePolicy", $write_policy] and .success == true))
' "$tmpdir/property-apply.json" >/dev/null

cmp "$tmpdir/property-apply.json" "$property_report" >/dev/null
vdo status --name "$vdo_name" > "$tmpdir/vdo-status-after-property.txt"
grep -Eiq "write[ -]?policy:[[:space:]]*$write_policy|write policy:[[:space:]]*$write_policy" "$tmpdir/vdo-status-after-property.txt"

jq -n --arg vdo_name "$vdo_name" '{
  version: 1,
  vdoVolumes: {
    ($vdo_name): {
      operation: "rescan"
    }
  }
}' > "$spec"

"$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"

jq -e --arg vdo_name "$vdo_name" '
  .status == "succeeded"
  and (.commandPlan[]
    | .commands
    | any(.argv == ["vdo", "status", "--name", $vdo_name])
    and any(.argv == ["vdostats", "--human-readable", $vdo_name])
    and any(.argv == ["disk-nix", "inspect", $vdo_name]))
  and (.executionResults
    | any(.argv == ["vdo", "status", "--name", $vdo_name] and .success == true)
    and any(.argv == ["vdostats", "--human-readable", $vdo_name] and .success == true)
    and any(.argv == ["disk-nix", "inspect", $vdo_name] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null

echo "VDO integration smoke test set write policy and rescanned $vdo_name"
