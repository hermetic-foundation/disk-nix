#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run iSCSI integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test rescans
real iSCSI sessions for the target provided through DISK_NIX_ISCSI_TARGET.
When DISK_NIX_LUN_PATH is set, it also rescans that host-visible LUN path.
When DISK_NIX_LUN_DATA_SURVIVAL=1 and DISK_NIX_LUN_MOUNTPOINT are set, it
writes a sentinel to an already-mounted filesystem on that LUN and verifies the
sentinel survives a failed-and-resumed host-LUN rescan. The harness does not
log in to or log out from targets.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "iSCSI integration smoke test must run as root" >&2
  exit 2
fi

target="${DISK_NIX_ISCSI_TARGET:-}"
lun_path="${DISK_NIX_LUN_PATH:-}"
lun_data_survival="${DISK_NIX_LUN_DATA_SURVIVAL:-0}"
lun_mountpoint="${DISK_NIX_LUN_MOUNTPOINT:-}"
if [[ -z "$target" ]]; then
  cat >&2 <<'MSG'
DISK_NIX_ISCSI_TARGET is required.

Example:
  DISK_NIX_ISCSI_TARGET=iqn.2026-06.example:storage.root
MSG
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" blockdev cmp findmnt install iscsiadm jq lsscsi multipath mountpoint; do
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
lun_spec="$tmpdir/lun-spec.json"
lun_report="$tmpdir/lun-apply-report.json"
failed_lun_report="$tmpdir/failed-lun-apply-report.json"
sentinel_expected="$tmpdir/iscsi-lun-sentinel.expected"

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

if [[ -n "$lun_path" ]]; then
  if [[ ! -e "$lun_path" ]]; then
    echo "host-side LUN path does not exist: $lun_path" >&2
    exit 2
  fi

  blockdev --getsize64 "$lun_path" >/dev/null

  jq -n --arg target "$target" --arg lun_path "$lun_path" '{
    version: 1,
    luns: {
      ($target + ":0"): {
        operation: "rescan",
        devices: [
          $lun_path
        ]
      }
    },
    apply: {
      allowOffline: true
    }
  }' > "$lun_spec"

  "$disk_nix_bin" apply \
    --spec "$lun_spec" \
    --execute \
    --report-out "$lun_report" \
    --json > "$tmpdir/lun-apply.json"

  jq -e --arg target "$target" --arg lun_path "$lun_path" '
    .status == "succeeded"
    and (.commandPlan[] | select(.actionId == ("luns:" + $target + ":0:rescan"))
      | .commands
      | any(.argv == ["iscsiadm", "--mode", "session", "--rescan"])
      and any(.argv == ["lsscsi", "-t", "-s"])
      and any(.argv == ["multipath", "-r"])
      and any(.argv == ["sh", "-c", "block=$(basename \"$(readlink -f \"$1\")\"); printf '\''1\\n'\'' > \"/sys/class/block/${block}/device/rescan\"", "disk-nix-scsi-rescan", $lun_path]))
    and (.executionResults
      | any(.argv == ["sh", "-c", "block=$(basename \"$(readlink -f \"$1\")\"); printf '\''1\\n'\'' > \"/sys/class/block/${block}/device/rescan\"", "disk-nix-scsi-rescan", $lun_path] and .success == true)
      and any(.argv == ["multipath", "-r"] and .success == true))
  ' "$tmpdir/lun-apply.json" >/dev/null

  cmp "$tmpdir/lun-apply.json" "$lun_report" >/dev/null

  if [[ "$lun_data_survival" == "1" ]]; then
    if [[ -z "$lun_mountpoint" ]]; then
      echo "DISK_NIX_LUN_MOUNTPOINT is required when DISK_NIX_LUN_DATA_SURVIVAL=1 is set" >&2
      exit 2
    fi
    if ! mountpoint -q "$lun_mountpoint"; then
      echo "host-side LUN mountpoint is not mounted: $lun_mountpoint" >&2
      exit 2
    fi
    findmnt --target "$lun_mountpoint" >/dev/null

    printf 'disk-nix iSCSI LUN sentinel %s %s\n' "$target" "$lun_path" > "$sentinel_expected"
    install -m 0600 "$sentinel_expected" "$lun_mountpoint/disk-nix-iscsi-lun-sentinel.txt"
    cmp "$sentinel_expected" "$lun_mountpoint/disk-nix-iscsi-lun-sentinel.txt" >/dev/null

    fail_tools="$tmpdir/fake-iscsi-lun-rescan-tools"
    mkdir -p "$fail_tools"
    real_sh="$(command -v sh)"
    cat > "$fail_tools/sh" <<EOF
#!/usr/bin/env bash
if [[ "\$*" == *"disk-nix-scsi-rescan $lun_path"* ]]; then
  echo "synthetic iSCSI LUN rescan failure for disk-nix data-survival coverage" >&2
  exit 77
fi
exec "$real_sh" "\$@"
EOF
    chmod +x "$fail_tools/sh"

    if PATH="$fail_tools:$PATH" "$disk_nix_bin" apply \
      --spec "$lun_spec" \
      --execute \
      --report-out "$failed_lun_report" \
      --json > "$tmpdir/failed-lun-apply.json"; then
      echo "expected injected host-side LUN rescan failure to fail apply" >&2
      exit 1
    fi

    jq -e --arg target "$target" --arg lun_path "$lun_path" '
      .status == "failed"
      and (.executionResults
        | any(
            .argv == ["sh", "-c", "block=$(basename \"$(readlink -f \"$1\")\"); printf '\''1\\n'\'' > \"/sys/class/block/${block}/device/rescan\"", "disk-nix-scsi-rescan", $lun_path]
            and .success == false
            and .statusCode == 77
            and (.stderr | contains("synthetic iSCSI LUN rescan failure"))
          ))
      and .partialExecutionRecovery.failedActionId == ("luns:" + $target + ":0:rescan")
      and (.partialExecutionRecovery.retryReviewActionIds | index("luns:" + $target + ":0:rescan") != null)
      and (.recoveryActions | any(.kind == "resume-after-fix"))
      and (.recoveryActions | any(.kind == "domain-recovery"))
    ' "$tmpdir/failed-lun-apply.json" >/dev/null

    cmp "$tmpdir/failed-lun-apply.json" "$failed_lun_report" >/dev/null
    cmp "$sentinel_expected" "$lun_mountpoint/disk-nix-iscsi-lun-sentinel.txt" >/dev/null

    "$disk_nix_bin" apply \
      --spec "$lun_spec" \
      --execute \
      --report-out "$tmpdir/resumed-lun-report.json" \
      --json > "$tmpdir/resumed-lun-apply.json"

    jq -e --arg target "$target" --arg lun_path "$lun_path" '
      .status == "succeeded"
      and (.executionResults
        | any(.argv == ["sh", "-c", "block=$(basename \"$(readlink -f \"$1\")\"); printf '\''1\\n'\'' > \"/sys/class/block/${block}/device/rescan\"", "disk-nix-scsi-rescan", $lun_path] and .success == true)
        and any(.argv == ["multipath", "-r"] and .success == true))
      and (.commandPlan[] | select(.actionId == ("luns:" + $target + ":0:rescan")))
    ' "$tmpdir/resumed-lun-apply.json" >/dev/null

    cmp "$tmpdir/resumed-lun-apply.json" "$tmpdir/resumed-lun-report.json" >/dev/null
    cmp "$sentinel_expected" "$lun_mountpoint/disk-nix-iscsi-lun-sentinel.txt" >/dev/null
  fi

  echo "iSCSI integration smoke test rescanned $target and host-side LUN $lun_path"
else
  echo "iSCSI integration smoke test rescanned $target"
fi
