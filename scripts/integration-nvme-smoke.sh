#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run NVMe integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test rescans
real NVMe namespace inventory for the controller provided through
DISK_NIX_NVME_CONTROLLER. When DISK_NIX_NVME_GROW=1 is set, it also executes a
reviewed namespace grow/rescan plan. When DISK_NIX_NVME_ATTACH_DETACH=1 is set,
it also attaches and detaches the disposable namespace selected by
DISK_NIX_NVME_NAMESPACE_ID and DISK_NIX_NVME_CONTROLLERS. The harness does not
create or delete namespaces.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "NVMe integration smoke test must run as root" >&2
  exit 2
fi

controller="${DISK_NIX_NVME_CONTROLLER:-}"
grow_namespace="${DISK_NIX_NVME_GROW:-0}"
attach_detach="${DISK_NIX_NVME_ATTACH_DETACH:-0}"
namespace_id="${DISK_NIX_NVME_NAMESPACE_ID:-}"
namespace_controllers="${DISK_NIX_NVME_CONTROLLERS:-}"
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

if [[ "$attach_detach" == "1" ]]; then
  if [[ -z "$namespace_id" ]] || [[ -z "$namespace_controllers" ]]; then
    cat >&2 <<'MSG'
DISK_NIX_NVME_NAMESPACE_ID and DISK_NIX_NVME_CONTROLLERS are required when
DISK_NIX_NVME_ATTACH_DETACH=1 is set.

Example:
  DISK_NIX_NVME_NAMESPACE_ID=7
  DISK_NIX_NVME_CONTROLLERS=0x1
MSG
    exit 2
  fi
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" cmp jq nvme; do
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
grow_spec="$tmpdir/grow-spec.json"
grow_report="$tmpdir/grow-report.json"
attach_spec="$tmpdir/attach-spec.json"
attach_report="$tmpdir/attach-report.json"
detach_spec="$tmpdir/detach-spec.json"
detach_report="$tmpdir/detach-report.json"

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

if [[ "$grow_namespace" == "1" ]]; then
  jq -n --arg controller "$controller" '{
    version: 1,
    nvmeNamespaces: {
      ($controller): {
        operation: "grow"
      }
    },
    apply: {
      allowGrow: true,
      allowOffline: true
    }
  }' > "$grow_spec"

  "$disk_nix_bin" apply \
    --spec "$grow_spec" \
    --execute \
    --report-out "$grow_report" \
    --json > "$tmpdir/grow-apply.json"

  jq -e --arg controller "$controller" '
    .status == "succeeded"
    and (.commandPlan[] | select(.actionId == ("nvmenamespaces:" + $controller + ":grow"))
      | .commands
      | any(.argv == ["nvme", "list-subsys", "--output-format=json"])
      and any(.argv == ["nvme", "ns-rescan", $controller]))
    and (.executionResults
      | any(.argv == ["nvme", "list-subsys", "--output-format=json"] and .success == true)
      and any(.argv == ["nvme", "ns-rescan", $controller] and .success == true))
  ' "$tmpdir/grow-apply.json" >/dev/null

  cmp "$tmpdir/grow-apply.json" "$grow_report" >/dev/null
  nvme list-ns "$controller" --all --output-format=json > "$tmpdir/list-ns-grown.json"
fi

if [[ "$attach_detach" == "1" ]]; then
  jq -n \
    --arg controller "$controller" \
    --arg namespace_id "$namespace_id" \
    --arg namespace_controllers "$namespace_controllers" \
    '{
      version: 1,
      nvmeNamespaces: {
        ($controller): {
          operation: "attach",
          namespaceId: $namespace_id,
          controllers: $namespace_controllers
        }
      },
      apply: {
        allowOffline: true
      }
    }' > "$attach_spec"

  "$disk_nix_bin" apply \
    --spec "$attach_spec" \
    --execute \
    --report-out "$attach_report" \
    --json > "$tmpdir/attach-apply.json"

  jq -e \
    --arg controller "$controller" \
    --arg namespace_id "$namespace_id" \
    --arg namespace_controllers "$namespace_controllers" \
    '
    .status == "succeeded"
    and (.commandPlan[] | select(.actionId == ("nvmenamespaces:" + $controller + ":attach"))
      | .commands
      | any(.argv == ["nvme", "list-subsys", "--output-format=json"])
      and any(.argv == ["nvme", "attach-ns", $controller, "--namespace-id", $namespace_id, "--controllers", $namespace_controllers])
      and any(.argv == ["nvme", "ns-rescan", $controller]))
    and (.executionResults
      | any(.argv == ["nvme", "attach-ns", $controller, "--namespace-id", $namespace_id, "--controllers", $namespace_controllers] and .success == true)
      and any(.argv == ["nvme", "ns-rescan", $controller] and .success == true))
  ' "$tmpdir/attach-apply.json" >/dev/null

  cmp "$tmpdir/attach-apply.json" "$attach_report" >/dev/null
  nvme list-ns "$controller" --all --output-format=json > "$tmpdir/list-ns-attached.json"

  jq -n \
    --arg controller "$controller" \
    --arg namespace_id "$namespace_id" \
    --arg namespace_controllers "$namespace_controllers" \
    '{
      version: 1,
      nvmeNamespaces: {
        ($controller): {
          operation: "detach",
          namespaceId: $namespace_id,
          controllers: $namespace_controllers
        }
      },
      apply: {
        allowOffline: true
      }
    }' > "$detach_spec"

  "$disk_nix_bin" apply \
    --spec "$detach_spec" \
    --execute \
    --report-out "$detach_report" \
    --json > "$tmpdir/detach-apply.json"

  jq -e \
    --arg controller "$controller" \
    --arg namespace_id "$namespace_id" \
    --arg namespace_controllers "$namespace_controllers" \
    '
    .status == "succeeded"
    and (.commandPlan[] | select(.actionId == ("nvmenamespaces:" + $controller + ":detach"))
      | .commands
      | any(.argv == ["nvme", "list-subsys", "--output-format=json"])
      and any(.argv == ["nvme", "detach-ns", $controller, "--namespace-id", $namespace_id, "--controllers", $namespace_controllers])
      and any(.argv == ["nvme", "ns-rescan", $controller]))
    and (.executionResults
      | any(.argv == ["nvme", "detach-ns", $controller, "--namespace-id", $namespace_id, "--controllers", $namespace_controllers] and .success == true)
      and any(.argv == ["nvme", "ns-rescan", $controller] and .success == true))
  ' "$tmpdir/detach-apply.json" >/dev/null

  cmp "$tmpdir/detach-apply.json" "$detach_report" >/dev/null
  nvme list-ns "$controller" --all --output-format=json > "$tmpdir/list-ns-detached.json"
fi

echo "NVMe integration smoke test rescanned $controller"
