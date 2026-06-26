#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run iSCSI integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test rescans
real iSCSI sessions for the target provided through DISK_NIX_ISCSI_TARGET.
The harness does not log in to or log out from targets.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "iSCSI integration smoke test must run as root" >&2
  exit 2
fi

target="${DISK_NIX_ISCSI_TARGET:-}"
if [[ -z "$target" ]]; then
  cat >&2 <<'MSG'
DISK_NIX_ISCSI_TARGET is required.

Example:
  DISK_NIX_ISCSI_TARGET=iqn.2026-06.example:storage.root
MSG
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" iscsiadm jq lsscsi; do
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

iscsiadm --mode session > "$tmpdir/session.txt"
if ! grep -F -- "$target" "$tmpdir/session.txt" >/dev/null; then
  echo "iSCSI target is not present in active sessions: $target" >&2
  exit 2
fi

lsscsi -t -s > "$tmpdir/lsscsi.txt"

"$disk_nix_bin" inspect "$target" --json > "$tmpdir/inspect.json"
jq -e --arg target "$target" '
  (.matchedNodes // .nodes // [])
  | any(
      .name == $target
      or .id == ("iscsi-target:" + $target)
      or (.properties // [] | any(.key == "iscsi.target" and .value == $target))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg target "$target" '{
  version: 1,
  iscsiSessions: {
    ($target): {
      operation: "rescan"
    }
  }
}' > "$spec"

"$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"

jq -e --arg target "$target" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("iscsisessions:" + $target + ":rescan"))
    | .commands
    | any(.argv == ["iscsiadm", "--mode", "session", "--rescan"])
    and any(.argv == ["lsscsi", "-t", "-s"])
    and any(.argv == ["disk-nix", "inspect", $target, "--json"]))
  and (.executionResults
    | any(.argv == ["iscsiadm", "--mode", "session", "--rescan"] and .success == true)
    and any(.argv == ["lsscsi", "-t", "-s"] and .success == true)
    and any(.argv == ["disk-nix", "inspect", $target, "--json"] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null

echo "iSCSI integration smoke test rescanned $target"
