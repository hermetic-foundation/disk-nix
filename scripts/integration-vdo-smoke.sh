#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run VDO integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test inspects
real VDO management state for the volume provided through DISK_NIX_VDO_NAME.
The exercised disk-nix apply operation is a read-only VDO rescan.
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

for tool in "$disk_nix_bin" jq vdo vdostats; do
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

echo "VDO integration smoke test rescanned $vdo_name"
