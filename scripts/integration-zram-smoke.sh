#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run zram integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test executes
real zram inventory commands. Zram property declarations are verified as
non-mutating generator reconciliation guidance; the harness does not recreate
or reconfigure live compressed swap devices.
MSG
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" jq swapon zramctl; do
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

spec="$tmpdir/zram-property-spec.json"
report="$tmpdir/zram-property-report.json"

jq -n '{
  version: 1,
  zram: {
    enable: true,
    swapDevices: 1,
    algorithm: "zstd",
    properties: {
      algorithm: "zstd",
      priority: 100
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$spec"

if ! "$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/zram-property-apply.json"; then
  cat "$tmpdir/zram-property-apply.json" >&2 || true
  cat "$report" >&2 || true
  exit 1
fi

jq -e '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "zram:inspect")
    | (.commands | length) >= 3
    and (.commands | all(.mutates == false and .readiness == "ready"))
    and (.commands | any(.argv == ["zramctl", "--bytes", "--raw", "--noheadings", "--output-all"]))
    and (.commands | any(.argv == ["swapon", "--show", "--bytes", "--raw"]))
    and (.commands | any(.argv == ["disk-nix", "zram"])))
  and (.commandPlan[] | select(.actionId == "zram:set-property:algorithm")
    | (.commands | all(.mutates == false and .readiness == "ready"))
    and (.notes | any(contains("services.disk-nix.zram"))))
  and (.commandPlan[] | select(.actionId == "zram:set-property:priority")
    | (.commands | all(.mutates == false and .readiness == "ready"))
    and (.notes | any(contains("zramSwap"))))
  and (.executionResults | any(.argv == ["zramctl", "--bytes", "--raw", "--noheadings", "--output-all"] and .success == true))
  and (.executionResults | any(.argv == ["swapon", "--show", "--bytes", "--raw"] and .success == true))
  and (.executionResults | any(.argv == ["disk-nix", "zram"] and .success == true))
' "$tmpdir/zram-property-apply.json" >/dev/null

cmp "$tmpdir/zram-property-apply.json" "$report" >/dev/null
zramctl --bytes --raw --noheadings --output-all >/dev/null
swapon --show --bytes --raw >/dev/null

echo "zram integration smoke test verified non-mutating property reconciliation inventory"
