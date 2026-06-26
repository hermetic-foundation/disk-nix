#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run layered VM integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates
a temporary loop-backed LUKS container, LVM volume group, logical volume, ext4
filesystem, and mount. It is intended for disposable VMs.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "layered VM integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" blockdev cryptsetup jq losetup lvcreate lvextend lvs mkfs.ext4 mount mountpoint pvcreate pvremove resize2fs truncate umount vgchange vgcreate vgremove vgs; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loopdev=""
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
lv_path="/dev/$vg/root"

printf 'disk-nix layered VM integration passphrase\n' > "$keyfile"
chmod 0600 "$keyfile"
mkdir -p "$mountpoint"
truncate --size 768M "$backing"
loopdev="$(losetup --find --show "$backing")"
cryptsetup luksFormat --batch-mode --key-file "$keyfile" "$loopdev"
cryptsetup open --key-file "$keyfile" "$loopdev" "$mapper"
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

before_size="$(blockdev --getsize64 "$lv_path")"
lvextend --yes --size 192M "$lv_path"
after_size="$(blockdev --getsize64 "$lv_path")"
if (( after_size <= before_size )); then
  echo "layered LV did not report growth after lvextend" >&2
  exit 1
fi

jq -n --arg lv_path "$lv_path" --arg mountpoint "$mountpoint" '{
  version: 1,
  filesystems: {
    layeredRoot: {
      device: $lv_path,
      fsType: "ext4",
      mountpoint: $mountpoint,
      resizePolicy: "grow-only"
    }
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

jq -e --arg lv_path "$lv_path" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == "filesystem:layeredRoot:grow")
    | .commands | any(.argv == ["resize2fs", $lv_path]))
  and (.executionResults | any(.argv == ["resize2fs", $lv_path] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null

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

echo "layered VM integration smoke test grew ext4 on $lv_path mounted at $mountpoint"
