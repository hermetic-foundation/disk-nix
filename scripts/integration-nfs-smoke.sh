#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run NFS client integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test mounts,
remounts, unmounts, and inspects a real NFS client mount against the export
provided through DISK_NIX_NFS_SOURCE. When DISK_NIX_NFS_EXPORT_PROPERTY=1 is
set, it also changes a temporary server-side export entry with exportfs.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "NFS client integration smoke test must run as root" >&2
  exit 2
fi

source="${DISK_NIX_NFS_SOURCE:-}"
if [[ -z "$source" ]]; then
  cat >&2 <<'MSG'
DISK_NIX_NFS_SOURCE is required.

Example:
  DISK_NIX_NFS_SOURCE=server.example.com:/srv/disk-nix-smoke
MSG
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"
fs_type="${DISK_NIX_NFS_FSTYPE:-nfs4}"
mount_options="${DISK_NIX_NFS_MOUNT_OPTIONS:-vers=4.2}"
remount_options="${DISK_NIX_NFS_REMOUNT_OPTIONS:-$mount_options}"
export_property="${DISK_NIX_NFS_EXPORT_PROPERTY:-0}"
export_client="${DISK_NIX_NFS_EXPORT_CLIENT:-127.0.0.1}"
export_options="${DISK_NIX_NFS_EXPORT_OPTIONS:-ro,sync,no_subtree_check}"

for tool in "$disk_nix_bin" findmnt grep jq mount mountpoint nfsstat umount; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done
if [[ "$export_property" == "1" ]] && ! command -v exportfs >/dev/null 2>&1; then
  echo "required tool is missing: exportfs" >&2
  exit 2
fi

tmpdir="$(mktemp -d)"
mounted=0

cleanup() {
  if [[ "$mounted" == "1" ]] && mountpoint -q "$tmpdir/mnt"; then
    umount "$tmpdir/mnt" || true
  fi
  if [[ "$export_property" == "1" ]] && [[ -d "$tmpdir/export" ]]; then
    exportfs -u "$export_client:$tmpdir/export" >/dev/null 2>&1 || true
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

mountpoint_path="$tmpdir/mnt"
rescan_spec="$tmpdir/rescan-spec.json"
remount_spec="$tmpdir/remount-spec.json"
export_property_spec="$tmpdir/export-property-spec.json"
rescan_report="$tmpdir/rescan-report.json"
remount_report="$tmpdir/remount-report.json"
export_property_report="$tmpdir/export-property-report.json"

mkdir -p "$mountpoint_path"
if [[ -n "$mount_options" ]]; then
  mount -t "$fs_type" -o "$mount_options" "$source" "$mountpoint_path"
else
  mount -t "$fs_type" "$source" "$mountpoint_path"
fi
mounted=1

"$disk_nix_bin" inspect "$mountpoint_path" --json > "$tmpdir/inspect.json"
jq -e --arg mountpoint_path "$mountpoint_path" --arg source "$source" '
  (.matchedNodes // .nodes // [])
  | any(
      .path == $mountpoint_path
      or .id == ("mount:" + $mountpoint_path)
      or (.properties // [] | any(.key == "nfs.source" and .value == $source))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n \
  --arg source "$source" \
  --arg mountpoint_path "$mountpoint_path" \
  --arg fs_type "$fs_type" \
  '{
    version: 1,
    nfs: {
      mounts: {
        ($mountpoint_path): {
          source: $source,
          fsType: $fs_type,
          operation: "rescan"
        }
      }
    }
  }' > "$rescan_spec"

"$disk_nix_bin" apply \
  --spec "$rescan_spec" \
  --execute \
  --report-out "$rescan_report" \
  --json > "$tmpdir/rescan-apply.json"

jq -e --arg mountpoint_path "$mountpoint_path" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("nfs.mounts:" + $mountpoint_path + ":rescan"))
    | .commands
    | any(.argv == ["findmnt", "--json", $mountpoint_path])
    and any(.argv == ["nfsstat", "-m", $mountpoint_path]))
  and (.executionResults
    | any(.argv == ["findmnt", "--json", $mountpoint_path] and .success == true)
    and any(.argv == ["nfsstat", "-m", $mountpoint_path] and .success == true))
' "$tmpdir/rescan-apply.json" >/dev/null
cmp "$tmpdir/rescan-apply.json" "$rescan_report" >/dev/null

jq -n \
  --arg source "$source" \
  --arg mountpoint_path "$mountpoint_path" \
  --arg fs_type "$fs_type" \
  --arg remount_options "$remount_options" \
  '{
    version: 1,
    nfs: {
      mounts: {
        ($mountpoint_path): {
          source: $source,
          fsType: $fs_type,
          operation: "remount",
          options: ($remount_options | split(",") | map(select(length > 0)))
        }
      }
    }
  }' > "$remount_spec"

"$disk_nix_bin" apply \
  --spec "$remount_spec" \
  --execute \
  --report-out "$remount_report" \
  --json > "$tmpdir/remount-apply.json"

jq -e --arg mountpoint_path "$mountpoint_path" --arg remount_options "$remount_options" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("nfs.mounts:" + $mountpoint_path + ":remount"))
    | .commands
    | any(.argv == ["mount", "-o", ("remount," + $remount_options), $mountpoint_path]))
  and (.executionResults
    | any(.argv == ["mount", "-o", ("remount," + $remount_options), $mountpoint_path] and .success == true))
' "$tmpdir/remount-apply.json" >/dev/null
cmp "$tmpdir/remount-apply.json" "$remount_report" >/dev/null

if [[ "$export_property" == "1" ]]; then
  mkdir -p "$tmpdir/export"
  jq -n \
    --arg export_path "$tmpdir/export" \
    --arg export_client "$export_client" \
    --arg export_options "$export_options" \
    '{
      version: 1,
      exports: {
        ($export_path): {
          client: $export_client,
          properties: {
            options: $export_options
          }
        }
      },
      apply: {
        allowOffline: true
      }
    }' > "$export_property_spec"

  "$disk_nix_bin" apply \
    --spec "$export_property_spec" \
    --execute \
    --report-out "$export_property_report" \
    --json > "$tmpdir/export-property-apply.json"

  jq -e \
    --arg export_path "$tmpdir/export" \
    --arg export_client "$export_client" \
    --arg export_options "$export_options" \
    '
    .status == "succeeded"
    and (.commandPlan[] | select(.actionId == ("exports:" + $export_path + ":set-property:options"))
      | .commands | any(.argv == ["exportfs", "-i", "-o", $export_options, ($export_client + ":" + $export_path)]))
    and (.executionResults
      | any(.argv == ["exportfs", "-i", "-o", $export_options, ($export_client + ":" + $export_path)] and .success == true))
  ' "$tmpdir/export-property-apply.json" >/dev/null

  cmp "$tmpdir/export-property-apply.json" "$export_property_report" >/dev/null
  exportfs -v | grep -F -- "$tmpdir/export" | grep -F -- "$export_client" >/dev/null
fi

findmnt --json "$mountpoint_path" >/dev/null
nfsstat -m "$mountpoint_path" >/dev/null

echo "NFS client integration smoke test rescanned and remounted $source at $mountpoint_path"
