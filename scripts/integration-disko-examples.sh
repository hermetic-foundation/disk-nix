#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
examples_dir="${DISK_NIX_DISKO_EXAMPLES_DIR:-$repo_root/examples/disko}"
disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"
execute="${DISK_NIX_DISKO_E2E_EXECUTE:-0}"
confirm="${DISK_NIX_DISKO_E2E_CONFIRM:-}"
required_confirm="wipe-/dev/sdb-/dev/sdc-/dev/sdd-/dev/sde-/dev/sdf"
test_disks=(/dev/sdb /dev/sdc /dev/sdd /dev/sde /dev/sdf)
e2e_root="${DISK_NIX_DISKO_E2E_ROOT:-/mnt/disk-nix-disko-e2e}"
execute_specs_dir=""

cleanup_storage() {
  local spec="${1:-}"
  if [[ -d "$e2e_root" ]]; then
    findmnt -R -rn -o TARGET "$e2e_root" 2>/dev/null | sort -r | while read -r mountpoint; do
      umount -fl "$mountpoint" 2>/dev/null || true
    done
  fi
  if [[ -n "$spec" && -f "$spec" ]]; then
    jq -r '.swaps[]?.device // empty' "$spec" | while read -r device; do
      swapoff "$device" 2>/dev/null || true
    done
    if command -v zpool >/dev/null; then
      jq -r '.pools | keys[]? // empty' "$spec" | while read -r pool; do
        zpool destroy -f "$pool" 2>/dev/null || true
      done
    fi
    if command -v vgremove >/dev/null; then
      jq -r '.volumeGroups | keys[]? // empty' "$spec" | while read -r group; do
        vgremove -ff -y "$group" 2>/dev/null || true
      done
    fi
    if command -v mdadm >/dev/null; then
      jq -r '.mdRaids[]?.target // empty' "$spec" | while read -r array; do
        mdadm --stop "$array" 2>/dev/null || true
      done
    fi
    if command -v cryptsetup >/dev/null; then
      jq -r '.luks.devices | keys[]? // empty' "$spec" | while read -r name; do
        cryptsetup close "$name" 2>/dev/null || true
      done
    fi
  fi
}

rewrite_spec_for_execute() {
  local input="$1"
  local output="$2"
  local name="$3"
  local root="$e2e_root/$name"
  jq --arg root "$root" '
    def remap_path:
      if type == "string" and startswith("/") then
        if . == "/" then $root else $root + . end
      else
        .
      end;
    .filesystems |= ((. // {}) | with_entries(.value.mountpoint |= remap_path))
    | .pools |= ((. // {}) | with_entries(.value.mountpoint |= remap_path))
    | .datasets |= ((. // {}) | with_entries(.value.mountpoint |= remap_path))
    | .btrfsSubvolumes |= ((. // {}) | with_entries(
        .value.mountpoint |= remap_path
        | .value.target |= remap_path
      ))
  ' "$input" >"$output"
}

# shellcheck disable=SC2329
cleanup_on_exit() {
  if [[ "$execute" == "1" ]]; then
    cleanup_storage
    if [[ -n "$execute_specs_dir" ]]; then
      rm -rf "$execute_specs_dir"
    fi
  fi
}

trap cleanup_on_exit EXIT

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
  mkdir -p "$e2e_root"
  execute_specs_dir="$(mktemp -d)"
fi

fail=0
for spec in "$examples_dir"/*.json; do
  [[ "$(basename "$spec")" == "manifest.json" ]] && continue
  spec_name="$(basename "$spec" .json)"
  run_spec="$spec"
  if [[ "$execute" == "1" ]]; then
    run_spec="$execute_specs_dir/$(basename "$spec")"
    rewrite_spec_for_execute "$spec" "$run_spec" "$spec_name"
    cleanup_storage "$run_spec"
    wipefs --all --force "${test_disks[@]}" >/dev/null 2>&1 || true
  fi
  echo "== $(basename "$spec")"

  plan_json="$(mktemp)"
  apply_json="$(mktemp)"
  if ! "$disk_nix_bin" plan --spec "$run_spec" --json >"$plan_json"; then
    echo "plan failed for $spec" >&2
    fail=1
    continue
  fi

  apply_args=(apply --spec "$run_spec" --json)
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
  if [[ "$execute" == "1" ]]; then
    cleanup_storage "$run_spec"
  fi
done

exit "$fail"
