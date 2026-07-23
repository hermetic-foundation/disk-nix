#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run failure-recovery integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this harness drives
disk-nix in execute mode. This test uses fake storage tools in a temporary
directory and does not mutate real block devices.
MSG
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" jq mktemp chmod mkdir rm cmp; do
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


script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
scenario_dir="${DISK_NIX_FAILURE_RECOVERY_SCENARIO_DIR:-$script_dir/integration-failure-recovery-smoke.d}"

for scenario_chunk in "$scenario_dir"/*.sh; do
  # shellcheck source=/dev/null
  source "$scenario_chunk"
done
