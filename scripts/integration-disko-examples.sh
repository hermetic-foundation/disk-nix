#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
examples_dir="${DISK_NIX_DISKO_EXAMPLES_DIR:-$repo_root/examples/disko}"
disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"
execute="${DISK_NIX_DISKO_E2E_EXECUTE:-0}"
confirm="${DISK_NIX_DISKO_E2E_CONFIRM:-}"
required_confirm="wipe-/dev/sdb-/dev/sdc-/dev/sdd-/dev/sde-/dev/sdf"
test_disks=(/dev/sdb /dev/sdc /dev/sdd /dev/sde /dev/sdf)

if [[ ! -d "$examples_dir" ]]; then
  echo "examples directory not found: $examples_dir" >&2
  exit 1
fi

if [[ "$execute" == "1" ]]; then
  if [[ "$confirm" != "$required_confirm" ]]; then
    echo "refusing destructive run" >&2
    echo "set DISK_NIX_DISKO_E2E_CONFIRM=$required_confirm to wipe /dev/sdb through /dev/sdf" >&2
    exit 1
  fi
  if [[ "$(id -u)" != "0" ]]; then
    echo "destructive E2E requires root" >&2
    exit 1
  fi
  for disk in "${test_disks[@]}"; do
    if [[ ! -b "$disk" ]]; then
      echo "required test disk is missing: $disk" >&2
      exit 1
    fi
    if lsblk -nr -o MOUNTPOINTS "$disk" | grep -q '[^[:space:]]'; then
      echo "refusing because $disk or a child has a mountpoint" >&2
      lsblk -o NAME,PATH,SIZE,TYPE,FSTYPE,MOUNTPOINTS "$disk" >&2
      exit 1
    fi
  done
  lsblk -o NAME,PATH,SIZE,TYPE,FSTYPE,MOUNTPOINTS "${test_disks[@]}"
fi

fail=0
for spec in "$examples_dir"/*.json; do
  [[ "$(basename "$spec")" == "manifest.json" ]] && continue
  echo "== $(basename "$spec")"

  plan_json="$(mktemp)"
  apply_json="$(mktemp)"
  if ! "$disk_nix_bin" plan --spec "$spec" --json >"$plan_json"; then
    echo "plan failed for $spec" >&2
    fail=1
    continue
  fi

  apply_args=(apply --spec "$spec" --json)
  if [[ "$execute" == "1" ]]; then
    apply_args+=(--execute)
  fi
  if ! "$disk_nix_bin" "${apply_args[@]}" >"$apply_json"; then
    echo "apply failed for $spec" >&2
    cat "$apply_json" >&2 || true
    fail=1
    continue
  fi

  jq -r '"commands=\(.commandSummary.commandCount) ready=\(.commandSummary.readyCount) missingDomain=\(.commandSummary.needsDomainImplementationCount) manualOnly=\(.commandSummary.manualOnlyCount) blocked=\(.apply.blockedCount)"' "$apply_json"
  if jq -e '.apply.blockedCount != 0 or .commandSummary.needsDomainImplementationCount != 0 or .commandSummary.manualOnlyCount != 0 or .commandSummary.readyCount != .commandSummary.commandCount' "$apply_json" >/dev/null; then
    echo "non-ready command plan for $spec" >&2
    jq '.apply.blockedSummary, .commandSummary, [.commandPlan[] | {actionId, notReady: [.commands[] | select(.readiness != "ready")]} | select(.notReady|length>0)]' "$apply_json" >&2
    fail=1
  fi
done

exit "$fail"
