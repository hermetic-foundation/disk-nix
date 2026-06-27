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
DISK_NIX_NVME_NAMESPACE_ID and DISK_NIX_NVME_CONTROLLERS. When
DISK_NIX_NVME_CREATE_DELETE=1 is set, it creates, attaches, detaches, and
deletes the disposable namespace selected by DISK_NIX_NVME_NAMESPACE_ID,
DISK_NIX_NVME_NAMESPACE_SIZE, and DISK_NIX_NVME_CONTROLLERS. When
DISK_NIX_NVME_RECONNECT=1 is set, it disconnects and reconnects the reviewed
controller target selected by DISK_NIX_NVME_RECONNECT_NQN,
DISK_NIX_NVME_RECONNECT_TRANSPORT, DISK_NIX_NVME_RECONNECT_TRADDR, optional
DISK_NIX_NVME_RECONNECT_TRSVCID, and optional
DISK_NIX_NVME_RECONNECT_CONTROLLER.
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
create_delete="${DISK_NIX_NVME_CREATE_DELETE:-0}"
reconnect_controller="${DISK_NIX_NVME_RECONNECT:-0}"
namespace_id="${DISK_NIX_NVME_NAMESPACE_ID:-}"
namespace_size="${DISK_NIX_NVME_NAMESPACE_SIZE:-}"
namespace_controllers="${DISK_NIX_NVME_CONTROLLERS:-}"
reconnect_nqn="${DISK_NIX_NVME_RECONNECT_NQN:-}"
reconnect_transport="${DISK_NIX_NVME_RECONNECT_TRANSPORT:-}"
reconnect_traddr="${DISK_NIX_NVME_RECONNECT_TRADDR:-}"
reconnect_trsvcid="${DISK_NIX_NVME_RECONNECT_TRSVCID:-}"
reconnect_expected_controller="${DISK_NIX_NVME_RECONNECT_CONTROLLER:-$controller}"
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
if [[ "$create_delete" == "1" ]]; then
  if [[ -z "$namespace_id" ]] || [[ -z "$namespace_size" ]] || [[ -z "$namespace_controllers" ]]; then
    cat >&2 <<'MSG'
DISK_NIX_NVME_NAMESPACE_ID, DISK_NIX_NVME_NAMESPACE_SIZE, and
DISK_NIX_NVME_CONTROLLERS are required when DISK_NIX_NVME_CREATE_DELETE=1 is
set.

Example:
  DISK_NIX_NVME_NAMESPACE_ID=7
  DISK_NIX_NVME_NAMESPACE_SIZE=1G
  DISK_NIX_NVME_CONTROLLERS=0x1
MSG
    exit 2
  fi
fi
if [[ "$reconnect_controller" == "1" ]]; then
  if [[ -z "$reconnect_nqn" ]] || [[ -z "$reconnect_transport" ]] || [[ -z "$reconnect_traddr" ]] || [[ -z "$reconnect_expected_controller" ]]; then
    cat >&2 <<'MSG'
DISK_NIX_NVME_RECONNECT_NQN, DISK_NIX_NVME_RECONNECT_TRANSPORT,
DISK_NIX_NVME_RECONNECT_TRADDR, and DISK_NIX_NVME_RECONNECT_CONTROLLER are
required when DISK_NIX_NVME_RECONNECT=1 is set.

Example:
  DISK_NIX_NVME_RECONNECT_NQN=nqn.2014-08.org.nvmexpress.discovery
  DISK_NIX_NVME_RECONNECT_TRANSPORT=tcp
  DISK_NIX_NVME_RECONNECT_TRADDR=192.0.2.10
  DISK_NIX_NVME_RECONNECT_TRSVCID=4420
  DISK_NIX_NVME_RECONNECT_CONTROLLER=/dev/nvme0
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

namespace_present() {
  local inventory="$1"
  local expected_id="$2"
  jq -e --arg expected_id "$expected_id" '
    [
      .. | objects
      | (.nsid? // .NSID? // .namespace? // .Namespace? // .NameSpace? // .nsid_decimal? // empty)
      | tostring
    ]
    | index($expected_id) != null
  ' "$inventory" >/dev/null
}

spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"
grow_spec="$tmpdir/grow-spec.json"
grow_report="$tmpdir/grow-report.json"
create_spec="$tmpdir/create-spec.json"
create_report="$tmpdir/create-report.json"
destroy_spec="$tmpdir/destroy-spec.json"
destroy_report="$tmpdir/destroy-report.json"
attach_spec="$tmpdir/attach-spec.json"
attach_report="$tmpdir/attach-report.json"
detach_spec="$tmpdir/detach-spec.json"
detach_report="$tmpdir/detach-report.json"
reconnect_spec="$tmpdir/reconnect-spec.json"
reconnect_report="$tmpdir/reconnect-report.json"

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

if [[ "$create_delete" == "1" ]]; then
  jq -n \
    --arg controller "$controller" \
    --arg namespace_id "$namespace_id" \
    --arg namespace_size "$namespace_size" \
    --arg namespace_controllers "$namespace_controllers" \
    '{
      version: 1,
      nvmeNamespaces: {
        ($controller): {
          operation: "create",
          desiredSize: $namespace_size,
          namespaceId: $namespace_id,
          controllers: $namespace_controllers
        }
      },
      apply: {
        allowDestructive: true,
        allowOffline: true
      }
    }' > "$create_spec"

  "$disk_nix_bin" apply \
    --spec "$create_spec" \
    --execute \
    --report-out "$create_report" \
    --json > "$tmpdir/create-apply.json"

  jq -e \
    --arg controller "$controller" \
    --arg namespace_id "$namespace_id" \
    --arg namespace_size "$namespace_size" \
    --arg namespace_controllers "$namespace_controllers" \
    '
    .status == "succeeded"
    and (.commandPlan[] | select(.actionId == ("nvmenamespaces:" + $controller + ":create"))
      | .commands
      | any(.argv == ["nvme", "list-ns", $controller, "--all", "--output-format=json"])
      and any(.argv == ["nvme", "create-ns", $controller, "--nsze-si", $namespace_size, "--ncap-si", $namespace_size])
      and any(.argv == ["nvme", "attach-ns", $controller, "--namespace-id", $namespace_id, "--controllers", $namespace_controllers])
      and any(.argv == ["nvme", "ns-rescan", $controller]))
    and (.executionResults
      | any(.argv == ["nvme", "create-ns", $controller, "--nsze-si", $namespace_size, "--ncap-si", $namespace_size] and .success == true)
      and any(.argv == ["nvme", "attach-ns", $controller, "--namespace-id", $namespace_id, "--controllers", $namespace_controllers] and .success == true)
      and any(.argv == ["nvme", "ns-rescan", $controller] and .success == true))
  ' "$tmpdir/create-apply.json" >/dev/null

  cmp "$tmpdir/create-apply.json" "$create_report" >/dev/null
  nvme list-ns "$controller" --all --output-format=json > "$tmpdir/list-ns-created.json"
  if ! namespace_present "$tmpdir/list-ns-created.json" "$namespace_id"; then
    echo "NVMe namespace identity drift: namespace $namespace_id was not visible after create" >&2
    exit 1
  fi

  jq -n \
    --arg controller "$controller" \
    --arg namespace_id "$namespace_id" \
    --arg namespace_controllers "$namespace_controllers" \
    '{
      version: 1,
      nvmeNamespaces: {
        ($controller): {
          destroy: true,
          namespaceId: $namespace_id,
          controllers: $namespace_controllers
        }
      },
      apply: {
        allowDestructive: true,
        allowOffline: true
      }
    }' > "$destroy_spec"

  "$disk_nix_bin" apply \
    --spec "$destroy_spec" \
    --execute \
    --report-out "$destroy_report" \
    --json > "$tmpdir/destroy-apply.json"

  jq -e \
    --arg controller "$controller" \
    --arg namespace_id "$namespace_id" \
    --arg namespace_controllers "$namespace_controllers" \
    '
    .status == "succeeded"
    and (.commandPlan[] | select(.actionId == ("nvmenamespaces:" + $controller + ":destroy"))
      | .commands
      | any(.argv == ["nvme", "list-subsys", "--output-format=json"])
      and any(.argv == ["nvme", "detach-ns", $controller, "--namespace-id", $namespace_id, "--controllers", $namespace_controllers])
      and any(.argv == ["nvme", "delete-ns", $controller, "--namespace-id", $namespace_id])
      and any(.argv == ["nvme", "ns-rescan", $controller]))
    and (.executionResults
      | any(.argv == ["nvme", "detach-ns", $controller, "--namespace-id", $namespace_id, "--controllers", $namespace_controllers] and .success == true)
      and any(.argv == ["nvme", "delete-ns", $controller, "--namespace-id", $namespace_id] and .success == true)
      and any(.argv == ["nvme", "ns-rescan", $controller] and .success == true))
  ' "$tmpdir/destroy-apply.json" >/dev/null

  cmp "$tmpdir/destroy-apply.json" "$destroy_report" >/dev/null
  nvme list-ns "$controller" --all --output-format=json > "$tmpdir/list-ns-deleted.json"
  if namespace_present "$tmpdir/list-ns-deleted.json" "$namespace_id"; then
    echo "NVMe namespace identity drift: namespace $namespace_id remained visible after delete" >&2
    exit 1
  fi
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

if [[ "$reconnect_controller" == "1" ]]; then
  connect_args=(connect -t "$reconnect_transport" -n "$reconnect_nqn" -a "$reconnect_traddr")
  if [[ -n "$reconnect_trsvcid" ]]; then
    connect_args+=(-s "$reconnect_trsvcid")
  fi

  nvme disconnect -n "$reconnect_nqn"
  nvme "${connect_args[@]}"

  for _ in $(seq 1 60); do
    if [[ -e "$reconnect_expected_controller" ]]; then
      break
    fi
    sleep 1
  done
  if [[ ! -e "$reconnect_expected_controller" ]]; then
    echo "NVMe reconnect did not expose expected controller: $reconnect_expected_controller" >&2
    exit 1
  fi

  "$disk_nix_bin" inspect "$reconnect_expected_controller" --json > "$tmpdir/reconnect-inspect.json"
  jq -e --arg controller "$reconnect_expected_controller" '
    (.matchedNodes // .nodes // [])
    | any(
        .path == $controller
        or .name == $controller
        or .id == ("block:" + $controller)
        or (.properties // [] | any(.key == "nvme.controller" and .value == $controller))
        or (.properties // [] | any(.key == "nvme.subsystem"))
      )
  ' "$tmpdir/reconnect-inspect.json" >/dev/null

  jq -n --arg controller "$reconnect_expected_controller" '{
    version: 1,
    nvmeNamespaces: {
      ($controller): {
        operation: "rescan"
      }
    }
  }' > "$reconnect_spec"

  "$disk_nix_bin" apply \
    --spec "$reconnect_spec" \
    --execute \
    --report-out "$reconnect_report" \
    --json > "$tmpdir/reconnect-apply.json"

  jq -e --arg controller "$reconnect_expected_controller" '
    .status == "succeeded"
    and (.commandPlan[] | select(.actionId == ("nvmenamespaces:" + $controller + ":rescan"))
      | .commands
      | any(.argv == ["nvme", "list-ns", $controller, "--all", "--output-format=json"])
      and any(.argv == ["nvme", "list-subsys", "--output-format=json"])
      and any(.argv == ["nvme", "ns-rescan", $controller]))
    and (.executionResults
      | any(.argv == ["nvme", "ns-rescan", $controller] and .success == true))
  ' "$tmpdir/reconnect-apply.json" >/dev/null

  cmp "$tmpdir/reconnect-apply.json" "$reconnect_report" >/dev/null
  nvme list-ns "$reconnect_expected_controller" --all --output-format=json > "$tmpdir/list-ns-reconnected.json"
fi

echo "NVMe integration smoke test rescanned $controller"
