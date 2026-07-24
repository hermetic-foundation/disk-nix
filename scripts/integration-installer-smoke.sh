#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INSTALLER_E2E_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run disk-nix installer E2E smoke test.

Set DISK_NIX_INSTALLER_E2E_DESTRUCTIVE=1 to acknowledge that this test wipes
and provisions DISK_NIX_INSTALLER_E2E_DISK with a disk-nix generated install
spec. This is intended for disposable VM disks.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "disk-nix installer E2E smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"
disk="${DISK_NIX_INSTALLER_E2E_DISK:-/dev/vdb}"
target="${DISK_NIX_INSTALLER_E2E_TARGET:-/mnt/disk-nix-installer-e2e}"
pool="${DISK_NIX_INSTALLER_E2E_POOL:-disknix_install_e2e}"

for tool in "$disk_nix_bin" jq mountpoint swapon wipefs zfs zpool; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
spec="$tmpdir/install.json"
plan_json="$tmpdir/plan.json"
apply_json="$tmpdir/apply.json"
apply_script="$tmpdir/apply.sh"
mount_script="$tmpdir/mount.sh"
nixos_script="$tmpdir/nixos-install.sh"

cleanup() {
  set +e
  swapoff /dev/disk/by-label/disknix-e2e-swap >/dev/null 2>&1
  if mountpoint -q "$target/boot"; then
    umount "$target/boot"
  fi
  for mountpoint_path in "$target/var/log" "$target/var" "$target/nix" "$target/home" "$target"; do
    if mountpoint -q "$mountpoint_path"; then
      umount "$mountpoint_path"
    fi
  done
  zpool destroy "$pool" >/dev/null 2>&1
  rm -rf "$target" "$tmpdir"
}
trap cleanup EXIT

check_jq() {
  local description=$1
  local filter=$2
  local file=$3
  if ! jq -e "$filter" "$file" >/dev/null; then
    echo "installer E2E assertion failed: $description"
    echo "file: $file"
    jq . "$file" || cat "$file"
    exit 1
  fi
}

"$disk_nix_bin" install template zfs-root \
  --disk "$disk" \
  --pool "$pool" \
  --boot-label E2E-BOOT \
  --swap-label disknix-e2e-swap \
  --swap-end 1153MiB \
  --zfs-start 1153MiB \
  --part-prefix "$disk" \
  --out "$spec"
echo "installer E2E: rendered template"

check_jq "rendered install spec carries requested disk and pool" '
  .version == 1
  and .apply.mode == "install"
  and .apply.allowDestructive == true
  and .install.kind == "nixos-zfs-root"
  and .install.zfs.pool == "'"$pool"'"
  and (.disks["'"$disk"'"].operation == "create")
' "$spec"
echo "installer E2E: validated rendered template"

"$disk_nix_bin" plan --spec "$spec" --json > "$plan_json"
echo "installer E2E: rendered plan"
check_jq "installer plan includes disk, boot, swap, pool, and dataset actions" '
  .summary.destructiveCount >= 3
  and (.actions | any(.id | startswith("disks:")))
  and (.actions | any(.id == "filesystem:boot:preserve-data-disabled"))
  and (.actions | any(.id == "swaps:disk:format"))
  and (.actions | any(.id | startswith("pools:")))
  and (.actions | any(.id | startswith("datasets:")))
' "$plan_json"
echo "installer E2E: validated plan"

"$disk_nix_bin" apply \
  --spec "$spec" \
  --script-out "$apply_script" \
  --report-out "$tmpdir/apply-report.json" \
  --json > "$tmpdir/apply-dry-run.json"
echo "installer E2E: rendered dry-run apply script"

grep -F "parted -s $disk mklabel gpt" "$apply_script" >/dev/null
grep -F "mkfs.vfat" "$apply_script" >/dev/null
grep -F "mkswap" "$apply_script" >/dev/null
grep -F "zpool create" "$apply_script" >/dev/null
grep -F "zfs create" "$apply_script" >/dev/null

if ! "$disk_nix_bin" apply --spec "$spec" --execute --json > "$apply_json"; then
  echo "installer E2E apply execution failed"
  jq . "$apply_json" || cat "$apply_json"
  exit 1
fi
echo "installer E2E: executed apply"
check_jq "installer apply executed all core storage commands" '
  .status == "succeeded"
  and (.executionResults | all(.success == true))
  and (.executionResults | any(.argv[0] == "parted"))
  and (.executionResults | any(.argv[0] == "mkfs.vfat"))
  and (.executionResults | any(.argv[0] == "mkswap"))
  and (.executionResults | any(.argv[0] == "zpool"))
  and (.executionResults | any(.argv[0] == "zfs"))
' "$apply_json"
echo "installer E2E: validated apply execution"

zpool status "$pool" >/dev/null
zfs list -H "$pool/root" >/dev/null
zfs list -H "$pool/root/home" >/dev/null
zfs list -H "$pool/root/nix" >/dev/null
zfs list -H "$pool/root/var" >/dev/null
zfs list -H "$pool/root/log" >/dev/null

"$disk_nix_bin" install mount --spec "$spec" --target "$target" --script-out "$mount_script"
echo "installer E2E: rendered mount script"
grep -F "zpool import -R \"\$target\" '$pool'" "$mount_script" >/dev/null
grep -F "if [[ -e '/dev/disk/by-label/E2E-BOOT' ]]; then" "$mount_script" >/dev/null
grep -F "mount '${disk}1' \"\$target/boot\"" "$mount_script" >/dev/null
grep -F "if [[ -e '/dev/disk/by-label/disknix-e2e-swap' ]]; then" "$mount_script" >/dev/null
grep -F "swapon '${disk}2'" "$mount_script" >/dev/null

"$mount_script"
echo "installer E2E: executed mount script"

mountpoint -q "$target"
mountpoint -q "$target/home"
mountpoint -q "$target/nix"
mountpoint -q "$target/var"
mountpoint -q "$target/var/log"
mountpoint -q "$target/boot"
if [[ -e /dev/disk/by-label/disknix-e2e-swap ]]; then
  swap_device="$(readlink -f /dev/disk/by-label/disknix-e2e-swap)"
else
  swap_device="${disk}2"
fi
swapon --show=NAME --noheadings | grep -F "$swap_device" >/dev/null

"$disk_nix_bin" install nixos \
  --spec "$spec" \
  --flake .#disk-nix-installer-e2e \
  --target "$target" \
  --script-out "$nixos_script"
echo "installer E2E: rendered nixos-install script"

grep -F "nixos-install --root \"\$target\" --flake '.#disk-nix-installer-e2e'" "$nixos_script" >/dev/null

echo "disk-nix installer E2E smoke test provisioned and mounted $pool on $disk"
