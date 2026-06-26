#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run multipath integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test reloads
real multipath maps for the map provided through DISK_NIX_MULTIPATH_MAP.
The harness does not add, remove, replace, flush, or resize paths.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "multipath integration smoke test must run as root" >&2
  exit 2
fi

map="${DISK_NIX_MULTIPATH_MAP:-}"
if [[ -z "$map" ]]; then
  cat >&2 <<'MSG'
DISK_NIX_MULTIPATH_MAP is required.

Example:
  DISK_NIX_MULTIPATH_MAP=mpatha
MSG
  exit 2
fi

case "$map" in
  mpath* | /dev/mapper/*) ;;
  *)
    echo "DISK_NIX_MULTIPATH_MAP must be an mpath* name or /dev/mapper/* path: $map" >&2
    exit 2
    ;;
esac

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" jq lsscsi multipath; do
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

multipath -ll "$map" > "$tmpdir/multipath-before.txt"
lsscsi -t -s > "$tmpdir/lsscsi.txt"

"$disk_nix_bin" inspect "$map" --json > "$tmpdir/inspect.json"
jq -e --arg map "$map" '
  (.matchedNodes // .nodes // [])
  | any(
      .name == $map
      or .path == $map
      or .id == ("multipath:" + $map)
      or (.properties // [] | any(.key == "multipath.wwid"))
      or (.properties // [] | any(.key == "multipath.dm"))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg map "$map" '{
  version: 1,
  multipathMaps: {
    inventory: {
      target: $map,
      operation: "rescan"
    }
  }
}' > "$spec"

"$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"

jq -e --arg map "$map" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "multipathmaps:inventory:rescan")
    | .commands
    | ([.[] | select(.argv == ["multipath", "-ll", $map])] | length == 2)
    and any(.argv == ["lsscsi", "-t", "-s"])
    and any(.argv == ["multipath", "-r"]))
  and (.executionResults
    | ([.[] | select(.argv == ["multipath", "-ll", $map] and .success == true)] | length == 2)
    and any(.argv == ["lsscsi", "-t", "-s"] and .success == true)
    and any(.argv == ["multipath", "-r"] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null
multipath -ll "$map" > "$tmpdir/multipath-after.txt"

echo "multipath integration smoke test rescanned $map"
