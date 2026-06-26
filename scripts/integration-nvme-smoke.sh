#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run NVMe integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test rescans
real NVMe namespace inventory for the controller provided through
DISK_NIX_NVME_CONTROLLER. The harness does not create, attach, detach, or
delete namespaces.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "NVMe integration smoke test must run as root" >&2
  exit 2
fi

controller="${DISK_NIX_NVME_CONTROLLER:-}"
if [[ -z "$controller" ]]; then
  cat >&2 <<'MSG'
DISK_NIX_NVME_CONTROLLER is required.

Example:
  DISK_NIX_NVME_CONTROLLER=/dev/nvme0
MSG
  exit 2
fi

case "$controller" in
  /dev/nvme[0-9]*) ;;
  *)
    echo "DISK_NIX_NVME_CONTROLLER must be a controller path such as /dev/nvme0: $controller" >&2
    exit 2
    ;;
esac

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" jq nvme; do
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

nvme list-ns "$controller" --all --output-format=json > "$tmpdir/list-ns.json"
nvme list-subsys --output-format=json > "$tmpdir/list-subsys.json"

"$disk_nix_bin" inspect "$controller" --json > "$tmpdir/inspect.json"
jq -e --arg controller "$controller" '
  (.matchedNodes // .nodes // [])
  | any(
      .path == $controller
      or .name == $controller
      or .id == ("block:" + $controller)
      or (.properties // [] | any(.key == "nvme.controller" and .value == $controller))
      or (.properties // [] | any(.key == "nvme.subsystem"))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg controller "$controller" '{
  version: 1,
  nvmeNamespaces: {
    ($controller): {
      operation: "rescan"
    }
  }
}' > "$spec"

"$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"

jq -e --arg controller "$controller" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("nvmenamespaces:" + $controller + ":rescan"))
    | .commands
    | ([.[] | select(.argv == ["nvme", "list-ns", $controller, "--all", "--output-format=json"])] | length == 2)
    and any(.argv == ["nvme", "list-subsys", "--output-format=json"])
    and any(.argv == ["nvme", "ns-rescan", $controller]))
  and (.executionResults
    | ([.[] | select(.argv == ["nvme", "list-ns", $controller, "--all", "--output-format=json"] and .success == true)] | length == 2)
    and any(.argv == ["nvme", "list-subsys", "--output-format=json"] and .success == true)
    and any(.argv == ["nvme", "ns-rescan", $controller] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null
nvme list-ns "$controller" --all --output-format=json > "$tmpdir/list-ns-after.json"

echo "NVMe integration smoke test rescanned $controller"
