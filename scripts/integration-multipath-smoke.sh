#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run multipath integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test reloads
real multipath maps for the map provided through DISK_NIX_MULTIPATH_MAP.
When DISK_NIX_MULTIPATH_RESIZE=1 is set, it also asks multipathd to resize the
selected map. When DISK_NIX_MULTIPATH_ADD_PATH or
DISK_NIX_MULTIPATH_REMOVE_PATH is set, it mutates those explicitly named paths.
When DISK_NIX_MULTIPATH_FLUSH=1 is set, it flushes the selected map. The
harness does not replace paths.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "multipath integration smoke test must run as root" >&2
  exit 2
fi

map="${DISK_NIX_MULTIPATH_MAP:-}"
resize_map="${DISK_NIX_MULTIPATH_RESIZE:-0}"
add_path="${DISK_NIX_MULTIPATH_ADD_PATH:-}"
remove_path="${DISK_NIX_MULTIPATH_REMOVE_PATH:-}"
flush_map="${DISK_NIX_MULTIPATH_FLUSH:-0}"
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

for tool in "$disk_nix_bin" cmp jq lsscsi multipath multipathd; do
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
resize_spec="$tmpdir/resize-spec.json"
resize_report="$tmpdir/resize-apply-report.json"
paths_spec="$tmpdir/paths-spec.json"
paths_report="$tmpdir/paths-apply-report.json"
flush_spec="$tmpdir/flush-spec.json"
flush_report="$tmpdir/flush-apply-report.json"

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

if [[ "$resize_map" == "1" ]]; then
  jq -n --arg map "$map" '{
    version: 1,
    multipathMaps: {
      resize: {
        target: $map,
        operation: "grow"
      }
    },
    apply: {
      allowGrow: true,
      allowOffline: true
    }
  }' > "$resize_spec"

  "$disk_nix_bin" apply \
    --spec "$resize_spec" \
    --execute \
    --report-out "$resize_report" \
    --json > "$tmpdir/resize-apply.json"

  jq -e --arg map "$map" '
    .status == "succeeded"
    and (.commandPlan[] | select(.actionId == "multipathmaps:resize:grow")
      | .commands
      | any(.argv == ["multipath", "-ll", $map])
      and any(.argv == ["lsscsi", "-t", "-s"])
      and any(.argv == ["multipathd", "resize", "map", $map])
      and any(.argv == ["multipath", "-r"]))
    and (.executionResults
      | any(.argv == ["multipathd", "resize", "map", $map] and .success == true)
      and any(.argv == ["multipath", "-r"] and .success == true))
  ' "$tmpdir/resize-apply.json" >/dev/null

  cmp "$tmpdir/resize-apply.json" "$resize_report" >/dev/null
  multipath -ll "$map" > "$tmpdir/multipath-resized.txt"

  echo "multipath integration smoke test rescanned and resized $map"
else
  echo "multipath integration smoke test rescanned $map"
fi

if [[ -n "$add_path" || -n "$remove_path" ]]; then
  if [[ -n "$add_path" && ! -e "$add_path" ]]; then
    echo "multipath add path does not exist: $add_path" >&2
    exit 2
  fi
  if [[ -n "$remove_path" && ! -e "$remove_path" ]]; then
    echo "multipath remove path does not exist: $remove_path" >&2
    exit 2
  fi

  jq -n \
    --arg map "$map" \
    --arg add_path "$add_path" \
    --arg remove_path "$remove_path" \
    '{
      version: 1,
      multipathMaps: {
        paths: ({
          target: $map
        }
        + (if $add_path == "" then {} else { addDevices: [ $add_path ] } end)
        + (if $remove_path == "" then {} else { removeDevices: [ $remove_path ] } end))
      },
      apply: {
        allowOffline: true,
        allowDeviceReplacement: true,
        allowPotentialDataLoss: true
      }
    }' > "$paths_spec"

  "$disk_nix_bin" apply \
    --spec "$paths_spec" \
    --execute \
    --report-out "$paths_report" \
    --json > "$tmpdir/paths-apply.json"

  jq -e --arg map "$map" --arg add_path "$add_path" --arg remove_path "$remove_path" '
    .status == "succeeded"
    and (if $add_path == "" then true else
      (.commandPlan[] | select(.actionId == ("multipathMaps:paths:add-device:" + $add_path))
        | .commands
        | any(.argv == ["multipath", "-ll", $map])
        and any(.argv == ["multipathd", "add", "path", $add_path]))
      and (.executionResults
        | any(.argv == ["multipathd", "add", "path", $add_path] and .success == true))
    end)
    and (if $remove_path == "" then true else
      (.commandPlan[] | select(.actionId == ("multipathMaps:paths:remove-device:" + $remove_path))
        | .commands
        | any(.argv == ["multipath", "-ll", $map])
        and any(.argv == ["multipathd", "del", "path", $remove_path]))
      and (.executionResults
        | any(.argv == ["multipathd", "del", "path", $remove_path] and .success == true))
    end)
  ' "$tmpdir/paths-apply.json" >/dev/null

  cmp "$tmpdir/paths-apply.json" "$paths_report" >/dev/null
  multipath -ll "$map" > "$tmpdir/multipath-paths-mutated.txt"

  echo "multipath integration smoke test mutated selected paths for $map"
fi

if [[ "$flush_map" == "1" ]]; then
  jq -n --arg map "$map" '{
    version: 1,
    multipathMaps: {
      flush: {
        target: $map,
        destroy: true
      }
    },
    apply: {
      allowOffline: true,
      allowDestructive: true,
      backupVerified: true
    }
  }' > "$flush_spec"

  "$disk_nix_bin" apply \
    --spec "$flush_spec" \
    --execute \
    --report-out "$flush_report" \
    --json > "$tmpdir/flush-apply.json"

  jq -e --arg map "$map" '
    .status == "succeeded"
    and (.commandPlan[] | select(.actionId == "multipathmaps:flush:destroy")
      | .commands
      | any(.argv == ["multipath", "-ll", $map])
      and any(.argv == ["multipath", "-f", $map]))
    and (.executionResults
      | any(.argv == ["multipath", "-f", $map] and .success == true))
  ' "$tmpdir/flush-apply.json" >/dev/null

  cmp "$tmpdir/flush-apply.json" "$flush_report" >/dev/null

  echo "multipath integration smoke test flushed $map"
fi
