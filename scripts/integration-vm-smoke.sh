#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run VM destructive integration suite.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this suite runs
real storage mutation tests. It must be run in a disposable VM or with an
explicit VM override for lab automation.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "VM destructive integration suite must run as root" >&2
  exit 2
fi

if [[ "${DISK_NIX_INTEGRATION_ASSUME_VM:-}" != "1" ]]; then
  if ! command -v systemd-detect-virt >/dev/null 2>&1; then
    echo "systemd-detect-virt is required unless DISK_NIX_INTEGRATION_ASSUME_VM=1 is set" >&2
    exit 2
  fi
  if ! systemd-detect-virt --quiet --vm; then
    cat >&2 <<'MSG'
Refusing to run destructive integration suite outside a detected VM.

Run this inside a disposable virtual machine, or set
DISK_NIX_INTEGRATION_ASSUME_VM=1 only for controlled lab automation where the
host isolation boundary is provided externally.
MSG
    exit 2
  fi
fi

default_harnesses="loop btrfs swap layered-vm failure-recovery"
harnesses="${DISK_NIX_VM_HARNESSES:-$default_harnesses}"

run_harness() {
  case "$1" in
    loop)
      disk-nix-integration-loop-smoke
      ;;
    btrfs)
      disk-nix-integration-btrfs-smoke
      ;;
    bcachefs)
      disk-nix-integration-bcachefs-smoke
      ;;
    bcache)
      disk-nix-integration-bcache-smoke
      ;;
    luks)
      disk-nix-integration-luks-smoke
      ;;
    swap)
      disk-nix-integration-swap-smoke
      ;;
    zram)
      disk-nix-integration-zram-smoke
      ;;
    lvm)
      disk-nix-integration-lvm-smoke
      ;;
    mdraid)
      disk-nix-integration-mdraid-smoke
      ;;
    zfs)
      disk-nix-integration-zfs-smoke
      ;;
    nfs)
      disk-nix-integration-nfs-smoke
      ;;
    vdo)
      disk-nix-integration-vdo-smoke
      ;;
    iscsi)
      disk-nix-integration-iscsi-smoke
      ;;
    multipath)
      disk-nix-integration-multipath-smoke
      ;;
    nvme)
      disk-nix-integration-nvme-smoke
      ;;
    failure-recovery)
      disk-nix-integration-failure-recovery-smoke
      ;;
    layered-vm)
      disk-nix-integration-layered-vm-smoke
      ;;
    *)
      echo "unknown VM integration harness: $1" >&2
      exit 2
      ;;
  esac
}

for harness in $harnesses; do
  echo "running disk-nix VM integration harness: $harness"
  run_harness "$harness"
done

echo "disk-nix VM destructive integration suite passed: $harnesses"
