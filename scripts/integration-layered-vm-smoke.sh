#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run layered VM integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates
a temporary partitioned loop disk, LUKS container, LVM volume group, logical
volume, ext4 filesystem, and mount. It is intended for disposable VMs.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "layered VM integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" blockdev cmp cryptsetup findmnt grep growpart jq losetup lsblk lvcreate lvextend lvs mkfs.ext4 mount mountpoint parted partprobe pvcreate pvremove resize2fs truncate umount vgchange vgcreate vgremove vgs xfs_growfs; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loopdev=""
partition=""
mapper="disk_nix_layered_vm_$$"
vg="disk_nix_layered_vm_$$"
mountpoint="$tmpdir/mnt"

cleanup() {
  if mountpoint -q "$mountpoint"; then
    umount "$mountpoint" || true
  fi
  if vgs "$vg" >/dev/null 2>&1; then
    vgchange --activate n "$vg" >/dev/null 2>&1 || true
    vgremove --force --force --yes "$vg" >/dev/null 2>&1 || true
  fi
  if [[ -e "/dev/mapper/$mapper" ]]; then
    cryptsetup close "$mapper" || true
  fi
  if [[ -n "$loopdev" ]] && losetup --list "$loopdev" >/dev/null 2>&1; then
    losetup --detach "$loopdev"
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

backing="$tmpdir/disk-nix-layered-vm.img"
keyfile="$tmpdir/keyfile"
spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"
close_spec="$tmpdir/close-spec.json"
close_report="$tmpdir/close-report.json"
failure_spec="$tmpdir/failure-spec.json"
failure_report="$tmpdir/failure-report.json"
lv_path="/dev/$vg/root"
sentinel="$mountpoint/disk-nix-layered-sentinel"

printf 'disk-nix layered VM integration passphrase\n' > "$keyfile"
chmod 0600 "$keyfile"
mkdir -p "$mountpoint"
truncate --size 768M "$backing"
loopdev="$(losetup --find --show "$backing")"
parted -s "$loopdev" mklabel gpt
parted -s "$loopdev" mkpart primary 1MiB 640MiB
partprobe "$loopdev"
for _ in {1..50}; do
  if [[ -b "${loopdev}p1" ]]; then
    partition="${loopdev}p1"
    break
  fi
  if [[ -b "${loopdev}1" ]]; then
    partition="${loopdev}1"
    break
  fi
  sleep 0.1
done
if [[ -z "$partition" ]]; then
  echo "partition node did not appear for $loopdev" >&2
  lsblk "$loopdev" >&2 || true
  exit 1
fi

cryptsetup luksFormat --batch-mode --key-file "$keyfile" "$partition"
cryptsetup open --key-file "$keyfile" "$partition" "$mapper"
pvcreate --force --yes "/dev/mapper/$mapper"
vgcreate "$vg" "/dev/mapper/$mapper"
lvcreate --yes --size 128M --name root "$vg"
mkfs.ext4 -F -q "$lv_path"
mount "$lv_path" "$mountpoint"

"$disk_nix_bin" inspect "$lv_path" --json > "$tmpdir/inspect-before.json"
jq -e --arg mountpoint "$mountpoint" --arg lv_path "$lv_path" '
  (.matchedNodes // .nodes // [])
  | any(
      .path == $lv_path
      or (.properties // [] | any(.key == "mount.target" and .value == $mountpoint))
      or (.properties // [] | any(.key == "lvm.lv-path" and .value == $lv_path))
      or (.properties // [] | any(.key == "lvm.lv-name" and .value == "root"))
    )
' "$tmpdir/inspect-before.json" >/dev/null

printf 'disk-nix layered vm persistence check\n' > "$sentinel"
before_size="$(blockdev --getsize64 "$lv_path")"
truncate --size 1152M "$backing"
losetup --set-capacity "$loopdev"

jq -n \
  --arg loopdev "$loopdev" \
  --arg partition "$partition" \
  --arg mapper "$mapper" \
  --arg lv_path "$lv_path" \
  --arg mountpoint "$mountpoint" '{
  version: 1,
  partitions: {
    layeredPart: {
      operation: "grow",
      device: $loopdev,
      target: $partition,
      partitionNumber: 1
    }
  },
  luks: {
    devices: {
      layeredMapper: {
        operation: "grow",
        device: $partition,
        target: $mapper
      }
    }
  },
  volumes: {
    layeredRoot: {
      operation: "grow",
      target: $lv_path,
      desiredSize: "192M"
    }
  },
  filesystems: {
    layeredRoot: {
      device: $lv_path,
      fsType: "ext4",
      mountpoint: $mountpoint,
      resizePolicy: "grow-only"
    },
    layeredRootRemount: {
      fsType: "ext4",
      mountpoint: $mountpoint,
      operation: "remount",
      options: ["rw", "noatime"]
    }
  },
  apply: {
    allowGrow: true,
    allowOffline: true
  }
}' > "$spec"

if ! "$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"; then
  cat "$tmpdir/apply.json" >&2 || true
  cat "$report" >&2 || true
  exit 1
fi

jq -e \
  --arg loopdev "$loopdev" \
  --arg mapper "$mapper" \
  --arg lv_path "$lv_path" \
  --arg mountpoint "$mountpoint" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "partitions:layeredPart:grow")
    | .commands | any(.argv == ["growpart", $loopdev, "1"]))
  and (.commandPlan[] | select(.actionId == "luks.devices:layeredMapper:grow")
    | .commands | any(.argv == ["cryptsetup", "resize", $mapper]))
  and (.commandPlan[] | select(.actionId == "volumes:layeredRoot:grow")
    | .commands | any(.argv == ["lvextend", "--resizefs", "--size", "192M", $lv_path]))
  and (.commandPlan[] | select(.actionId == "filesystem:layeredRoot:grow")
    | .commands | any(.argv == ["resize2fs", $lv_path]))
  and (.commandPlan[] | select(.actionId == "filesystems:layeredRootRemount:remount")
    | .commands | any(.argv == ["mount", "-o", "remount,rw,noatime", $mountpoint]))
  and (.executionResults | any(.argv == ["growpart", $loopdev, "1"] and .success == true))
  and (.executionResults | any(.argv == ["cryptsetup", "resize", $mapper] and .success == true))
  and (.executionResults | any(.argv == ["lvextend", "--resizefs", "--size", "192M", $lv_path] and .success == true))
  and (.executionResults | any(.argv == ["resize2fs", $lv_path] and .success == true))
  and (.executionResults | any(.argv == ["mount", "-o", "remount,rw,noatime", $mountpoint] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null
after_size="$(blockdev --getsize64 "$lv_path")"
if (( after_size <= before_size )); then
  echo "layered LV did not report growth after multi-domain disk-nix apply" >&2
  exit 1
fi
findmnt -no OPTIONS "$mountpoint" | tr ',' '\n' | grep -qx noatime
printf 'disk-nix layered vm persistence check\n' | cmp - "$sentinel" >/dev/null

"$disk_nix_bin" inspect "$lv_path" --json > "$tmpdir/inspect-after.json"
jq -e --arg mountpoint "$mountpoint" --arg lv_path "$lv_path" '
  (.matchedNodes // .nodes // [])
  | any(
      .path == $lv_path
      or (.properties // [] | any(.key == "mount.target" and .value == $mountpoint))
      or (.properties // [] | any(.key == "lvm.lv-path" and .value == $lv_path))
      or (.properties // [] | any(.key == "lvm.lv-name" and .value == "root"))
    )
' "$tmpdir/inspect-after.json" >/dev/null

jq -n \
  --arg lv_path "$lv_path" \
  --arg mountpoint "$mountpoint" '{
  version: 1,
  volumes: {
    layeredFailureGrow: {
      operation: "grow",
      target: $lv_path,
      desiredSize: "256M"
    }
  },
  filesystems: {
    layeredFailureFilesystem: {
      operation: "grow",
      device: $lv_path,
      fsType: "xfs",
      mountpoint: $mountpoint,
      resizePolicy: "grow-only"
    },
    layeredFailureRemount: {
      fsType: "ext4",
      mountpoint: $mountpoint,
      operation: "remount",
      options: ["rw", "relatime"]
    }
  },
  apply: {
    allowGrow: true
  }
}' > "$failure_spec"

if "$disk_nix_bin" apply \
  --spec "$failure_spec" \
  --execute \
  --report-out "$failure_report" \
  --json > "$tmpdir/failure-apply.json"; then
  echo "expected layered VM failure injection to fail apply" >&2
  cat "$tmpdir/failure-apply.json" >&2 || true
  exit 1
fi

jq -e \
  --arg lv_path "$lv_path" \
  --arg mountpoint "$mountpoint" '
  .status == "failed"
  and .apply.blockedCount == 0
  and (.commandPlan[] | select(.actionId == "volumes:layeredFailureGrow:grow")
    | .commands | any(.argv == ["lvextend", "--resizefs", "--size", "256M", $lv_path]))
  and (.commandPlan[] | select(.actionId == "filesystem:layeredFailureFilesystem:grow")
    | .commands | any(.argv == ["xfs_growfs", $mountpoint]))
  and (.commandPlan[] | select(.actionId == "filesystems:layeredFailureRemount:remount")
    | .commands | any(.argv == ["mount", "-o", "remount,rw,relatime", $mountpoint]))
  and (.executionResults | any(.argv == ["lvextend", "--resizefs", "--size", "256M", $lv_path] and .success == true))
  and (.executionResults | any(.argv == ["xfs_growfs", $mountpoint] and .success == false and (.statusCode // 0) != 0))
  and .partialExecutionRecovery.completedActionIds == ["volumes:layeredFailureGrow:grow"]
  and .partialExecutionRecovery.failedActionId == "filesystem:layeredFailureFilesystem:grow"
  and .partialExecutionRecovery.failedPhase == "command"
  and .partialExecutionRecovery.failedCommand == ["xfs_growfs", $mountpoint]
  and .partialExecutionRecovery.retryReviewActionIds == ["filesystem:layeredFailureFilesystem:grow"]
  and (.partialExecutionRecovery.remainingActionIds | index("filesystems:layeredFailureRemount:remount") != null)
  and .partialExecutionRecovery.completedMutatingCommandCount >= 1
  and (.partialExecutionRecovery.notes | any(contains("fresh topology")))
  and (.recoveryActions | any(.kind == "domain-recovery"))
  and (.recoveryActions | any(.kind == "roll-forward-review"))
  and (.recoveryActions | any(.kind == "rollback-review"))
  and (.recoveryActions | any(.kind == "preserve-recovery-points"))
' "$tmpdir/failure-apply.json" >/dev/null

cmp "$tmpdir/failure-apply.json" "$failure_report" >/dev/null
after_failure_size="$(blockdev --getsize64 "$lv_path")"
if (( after_failure_size <= after_size )); then
  echo "layered LV did not report growth before injected VM apply failure" >&2
  exit 1
fi
printf 'disk-nix layered vm persistence check\n' | cmp - "$sentinel" >/dev/null

umount "$mountpoint"
vgchange --activate n "$vg"

jq -n --arg partition "$partition" --arg mapper "$mapper" '{
  version: 1,
  luks: {
    devices: {
      layeredMapper: {
        device: $partition,
        target: $mapper,
        operation: "close"
      }
    }
  },
  apply: {
    allowOffline: true
  }
}' > "$close_spec"

if ! "$disk_nix_bin" apply \
  --spec "$close_spec" \
  --execute \
  --report-out "$close_report" \
  --json > "$tmpdir/close-apply.json"; then
  cat "$tmpdir/close-apply.json" >&2 || true
  cat "$close_report" >&2 || true
  exit 1
fi

jq -e --arg mapper "$mapper" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "luks.devices:layeredMapper:close")
    | .commands | any(.argv == ["cryptsetup", "close", $mapper]))
  and (.executionResults
    | any(.argv == ["cryptsetup", "close", $mapper] and .success == true))
' "$tmpdir/close-apply.json" >/dev/null

cmp "$tmpdir/close-apply.json" "$close_report" >/dev/null
if [[ -e "/dev/mapper/$mapper" ]]; then
  echo "layered VM LUKS mapper still exists after disk-nix close operation" >&2
  exit 1
fi

cryptsetup open --key-file "$keyfile" "$partition" "$mapper"
vgchange --activate y "$vg"
mount "$lv_path" "$mountpoint"
printf 'disk-nix layered vm persistence check\n' | cmp - "$sentinel" >/dev/null

"$disk_nix_bin" inspect "$lv_path" --json > "$tmpdir/inspect-reopened.json"
jq -e --arg mountpoint "$mountpoint" --arg lv_path "$lv_path" '
  (.matchedNodes // .nodes // [])
  | any(
      .path == $lv_path
      or (.properties // [] | any(.key == "mount.target" and .value == $mountpoint))
      or (.properties // [] | any(.key == "lvm.lv-path" and .value == $lv_path))
      or (.properties // [] | any(.key == "lvm.lv-name" and .value == "root"))
    )
' "$tmpdir/inspect-reopened.json" >/dev/null

echo "layered VM integration smoke test grew partition, LUKS, LVM, ext4, remounted, closed LUKS through disk-nix, and reopened $lv_path mounted at $mountpoint"
