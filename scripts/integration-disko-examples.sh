#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
examples_dir="${DISK_NIX_DISKO_EXAMPLES_DIR:-$repo_root/examples/disko}"
disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"
execute="${DISK_NIX_DISKO_E2E_EXECUTE:-0}"
preflight="${DISK_NIX_DISKO_E2E_PREFLIGHT:-0}"
require_all_kernels="${DISK_NIX_DISKO_E2E_REQUIRE_ALL_KERNELS:-0}"
confirm="${DISK_NIX_DISKO_E2E_CONFIRM:-}"
default_test_disks=(
  /dev/disk/by-id/wwn-0x5000c500a5a461dc
  /dev/disk/by-id/wwn-0x5000c50087a102ce
  /dev/disk/by-id/wwn-0x5000c50087a11cd1
  /dev/disk/by-id/wwn-0x5000c500a5a40803
  /dev/disk/by-id/wwn-0x5000c500a5a3ab29
)
if [[ -n "${DISK_NIX_DISKO_E2E_DEVICES:-}" ]]; then
  read -r -a test_disks <<<"$DISK_NIX_DISKO_E2E_DEVICES"
else
  test_disks=("${default_test_disks[@]}")
fi
test_disk_list="${test_disks[*]}"
required_confirm="wipe-${test_disk_list// /-}"
e2e_root="${DISK_NIX_DISKO_E2E_ROOT:-/mnt/disk-nix-disko-e2e}"
e2e_passphrase="${DISK_NIX_DISKO_E2E_PASSPHRASE:-disk-nix-e2e-passphrase}"
execute_specs_dir=""
shim_dir=""

create_execute_shims() {
  shim_dir="$(mktemp -d)"
  local real_bcachefs real_blockdev real_cryptsetup real_mdadm real_mount real_parted real_pvcreate real_zfs
  real_bcachefs="$(command -v bcachefs || true)"
  real_blockdev="$(command -v blockdev)"
  real_cryptsetup="$(command -v cryptsetup || true)"
  real_mdadm="$(command -v mdadm || true)"
  real_mount="$(command -v mount)"
  real_parted="$(command -v parted)"
  real_pvcreate="$(command -v pvcreate || true)"
  real_zfs="$(command -v zfs || true)"

  if [[ -n "$real_bcachefs" ]]; then
    cat >"$shim_dir/bcachefs" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "device" && "\${2:-}" == "add" ]]; then
  exec "$real_bcachefs" device add --force "\${@:3}"
fi
exec "$real_bcachefs" "\$@"
EOF
  fi

cat >"$shim_dir/blockdev" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--rereadpt" ]]; then
  target="\${2:-}"
  "$real_blockdev" "\$@" || true
  if [[ "\$target" == /dev/md/* ]]; then
    name="\${target##*/}"
    actual="\$(readlink -f "\$target" 2>/dev/null || true)"
    if [[ -n "\$actual" ]]; then
      mkdir -p /dev/md
      for partition in "\${actual}"p*; do
        if [[ -e "\$partition" ]]; then
          suffix="\${partition#"\$actual"}"
          ln -sf "../\${partition#/dev/}" "/dev/md/\${name}\${suffix}"
        fi
      done
    fi
  fi
  exit 0
else
  exec "$real_blockdev" "\$@"
fi
EOF

  if [[ -n "$real_mdadm" ]]; then
    cat >"$shim_dir/mdadm" <<EOF
#!/usr/bin/env bash
if [[ "\${1:-}" == "--create" ]]; then
  target="\${2:-}"
  printf 'n\n' | "$real_mdadm" "\$@" --run --force --assume-clean
  status="\$?"
  if [[ "\$target" == /dev/md/* && ! -e "\$target" ]]; then
    name="\${target##*/}"
    mkdir -p /dev/md
    for array in /dev/md[0-9]*; do
      if [[ -e "\$array" ]] && "$real_mdadm" --detail "\$array" 2>/dev/null | grep -q "Name : .*:\$name"; then
        ln -sf "../\${array#/dev/}" "\$target"
        break
      fi
    done
  fi
  if [[ "\$status" != "0" && -n "\$target" ]] && "$real_mdadm" --detail "\$target" >/dev/null 2>&1; then
    exit 0
  fi
  exit "\$status"
fi
exec "$real_mdadm" "\$@"
EOF
  fi

  cat >"$shim_dir/mount" <<EOF
#!/usr/bin/env bash
target="\${@: -1}"
if [[ "\$target" == /* ]]; then
  mkdir -p -- "\$target"
fi
exec "$real_mount" "\$@"
EOF

  cat >"$shim_dir/parted" <<EOF
#!/usr/bin/env bash
needs_end_of_options=0
for arg in "\$@"; do
  if [[ "\$arg" =~ ^-[0-9] ]]; then
    needs_end_of_options=1
    break
  fi
done
if [[ "\$needs_end_of_options" == "1" ]]; then
  opts=()
  while [[ "\${1:-}" == -* && ! "\${1:-}" =~ ^-[0-9] ]]; do
    opts+=("\$1")
    shift
  done
  exec "$real_parted" "\${opts[@]}" -- "\$@"
fi
exec "$real_parted" "\$@"
EOF

  if [[ -n "$real_cryptsetup" ]]; then
    cat >"$shim_dir/cryptsetup" <<EOF
#!/usr/bin/env bash
passphrase="$e2e_passphrase"
case "\${1:-}" in
  isLuks)
    if [[ "\${2:-}" != /* && -e "/dev/mapper/\${2:-}" ]]; then
      exit 0
    fi
    exec "$real_cryptsetup" "\$@"
    ;;
  luksFormat)
    printf '%s\n' "\$passphrase" | exec "$real_cryptsetup" --batch-mode "\$@" -
    ;;
  open)
    shift
    printf '%s\n' "\$passphrase" | exec "$real_cryptsetup" open --key-file - "\$@"
    ;;
  *)
    exec "$real_cryptsetup" "\$@"
    ;;
esac
EOF
  fi

  if [[ -n "$real_pvcreate" ]]; then
    cat >"$shim_dir/pvcreate" <<EOF
#!/usr/bin/env bash
exec "$real_pvcreate" -ff -y "\$@"
EOF
  fi

  if [[ -n "$real_zfs" ]]; then
    cat >"$shim_dir/zfs" <<EOF
#!/usr/bin/env bash
passphrase="$e2e_passphrase"
if [[ "\${1:-}" == "create" ]]; then
  needs_passphrase=0
  for arg in "\$@"; do
    case "\$arg" in
      encryption=*|keyformat=passphrase|keylocation=prompt)
        needs_passphrase=1
        ;;
    esac
  done
  if [[ "\$needs_passphrase" == "1" ]]; then
    printf '%s\n%s\n' "\$passphrase" "\$passphrase" | exec "$real_zfs" "\$@"
  fi
fi
exec "$real_zfs" "\$@"
EOF
  fi

  chmod +x "$shim_dir"/*
  export PATH="$shim_dir:$PATH"
}

fail_current_spec() {
  local run_spec="${1:-}"
  fail=1
  if [[ "$execute" == "1" ]]; then
    cleanup_storage "$run_spec"
  fi
}

spec_requires_zfs() {
  local spec="$1"
  jq -e '((.pools // {}) | length > 0) or ((.datasets // {}) | length > 0) or ((.zvols // {}) | length > 0)' "$spec" >/dev/null
}

zfs_is_available() {
  command -v zpool >/dev/null && zpool list -H >/dev/null 2>&1
}

spec_requires_bcachefs() {
  local spec="$1"
  jq -e '(.filesystems // {}) | any(.fsType == "bcachefs")' "$spec" >/dev/null
}

bcachefs_is_available() {
  grep -qw bcachefs /proc/filesystems
}

wipe_test_disks() {
  for disk in "${test_disks[@]}"; do
    lsblk -nrpo PATH "$disk" 2>/dev/null | sort -r | while read -r path; do
      wipefs --all --force "$path" >/dev/null 2>&1 || true
    done
    blockdev --rereadpt "$disk" >/dev/null 2>&1 || true
  done
}

cleanup_storage() {
  local spec="${1:-}"
  if [[ -d "$e2e_root" ]]; then
    findmnt -rn -o TARGET 2>/dev/null | grep -F "$e2e_root" | sort -r | while read -r mountpoint; do
      umount -fl -- "$mountpoint" 2>/dev/null || true
    done || true
  fi
  if [[ -n "$spec" && -f "$spec" ]]; then
    jq -r '.swaps[]?.device // empty' "$spec" | while read -r device; do
      swapoff "$device" 2>/dev/null || true
    done
    if command -v zpool >/dev/null; then
      jq -r '(.pools // {}) | keys[]' "$spec" | while read -r pool; do
        zpool destroy -f "$pool" 2>/dev/null || true
      done
    fi
    if command -v vgremove >/dev/null; then
      jq -r '(.volumeGroups // {}) | keys[]' "$spec" | while read -r group; do
        vgremove -ff -y "$group" 2>/dev/null || true
      done
    fi
    if command -v mdadm >/dev/null; then
      jq -r '.mdRaids[]?.target // empty' "$spec" | while read -r array; do
        mdadm --stop "$array" 2>/dev/null || true
      done
    fi
    if command -v cryptsetup >/dev/null; then
      jq -r '(.luks.devices // {}) | keys[]' "$spec" | while read -r name; do
        cryptsetup close "$name" 2>/dev/null || true
      done
    fi
  fi
  if command -v zpool >/dev/null; then
    for pool in zroot storage storage2; do
      zpool destroy -f "$pool" 2>/dev/null || true
    done
  fi
  if command -v vgremove >/dev/null; then
    for group in pool mainpool; do
      vgchange -an "$group" 2>/dev/null || true
      vgremove -ff -y "$group" 2>/dev/null || true
    done
  fi
  if command -v cryptsetup >/dev/null; then
    for name in crypted crypted1 crypted2 p1 p2; do
      cryptsetup close "$name" 2>/dev/null || true
    done
  fi
  if command -v mdadm >/dev/null && [[ -r /proc/mdstat ]]; then
    awk '/^md/ {print "/dev/" $1}' /proc/mdstat | while read -r array; do
      mdadm --stop "$array" 2>/dev/null || true
    done
    for disk in "${test_disks[@]}"; do
      lsblk -nrpo PATH "$disk" 2>/dev/null | tail -n +2 | while read -r partition; do
        mdadm --zero-superblock --force "$partition" 2>/dev/null || true
      done
    done
  fi
}

rewrite_spec_for_execute() {
  local input="$1"
  local output="$2"
  local name="$3"
  local root="$e2e_root/$name"
  jq \
    --arg root "$root" \
    --arg disk_b "${test_disks[0]}" \
    --arg disk_c "${test_disks[1]}" \
    --arg disk_d "${test_disks[2]}" \
    --arg disk_e "${test_disks[3]}" \
    --arg disk_f "${test_disks[4]}" \
    '
    . as $original |
    def remap_disk($from; $to):
      if . == $from then
        $to
      elif startswith($from) and ((.[($from | length):]) | test("^p?[0-9]+$")) then
        $to + "-part" + ((.[($from | length):]) | sub("^p"; ""))
      else
        .
      end;
    def remap_device:
      if type == "string" then
        remap_disk("/dev/sdb"; $disk_b)
        | remap_disk("/dev/sdc"; $disk_c)
        | remap_disk("/dev/sdd"; $disk_d)
        | remap_disk("/dev/sde"; $disk_e)
        | remap_disk("/dev/sdf"; $disk_f)
      else
        .
      end;
    def remap_devices:
      walk(
        if type == "object" then
          with_entries(.key |= remap_device)
        elif type == "string" then
          remap_device
        else
          .
        end
      );
    def remap_path:
      if type == "string" and startswith("/") then
        if . == "/" then $root else $root + . end
      else
        .
      end;
    def remap_mountpoint:
      if type == "string" and . != "legacy" and . != "none" then
        if startswith("/") then
          if . == "/" then $root else $root + . end
        else
          $root + "/" + .
        end
      else
        .
      end;
    def absolute_mountpoint($path):
      if $path | startswith("/") then $path else "/" + $path end;
    def join_paths($base; $path):
      if $base == "/" then $path else $base + $path end;
    remap_devices
    | .filesystems |= ((. // {}) | with_entries(.value.mountpoint |= remap_mountpoint))
    | .pools |= ((. // {}) | with_entries(.value.mountpoint |= remap_mountpoint))
    | .datasets |= ((. // {}) | with_entries(.value.mountpoint |= remap_mountpoint))
    | .btrfsSubvolumes |= ((. // {}) | with_entries(
        (.value.metadata.parentFilesystem // null) as $parent
        | ($original.filesystems[$parent].mountpoint // null) as $parent_mount
        | .value.mountpoint |= remap_mountpoint
        | .value.target |= (
            if type == "string" and startswith("/") and $parent_mount != null then
              $root + join_paths(absolute_mountpoint($parent_mount); .)
            else
              remap_path
            end
          )
      ))
  ' "$input" >"$output"
}

validate_execute_plan_paths() {
  local apply_json="$1"
  local spec="$2"
  local allowed_disk_roots
  allowed_disk_roots="$(
    for disk in "${test_disks[@]}"; do
      printf '%s\n' "$disk"
    done | sort -u
  )"
  local unsafe
  unsafe="$(
    jq -r --arg root "$e2e_root" --arg allowed_disk_roots "$allowed_disk_roots" '
      def allowed_disks: $allowed_disk_roots | split("\n") | map(select(. != ""));
      def is_allowed_disk_path($path):
        any(allowed_disks[]; . as $disk
          | ($path == $disk)
            or (($path | startswith($disk + "-part")) and (($path[($disk + "-part" | length):]) | test("^[0-9]+$")))
        );
      .commandPlan[].commands[].argv[]?
      | select(type == "string" and startswith("/"))
      | . as $path
      | select(
          ($path == "/proc/mdstat")
          or ($path | startswith($root + "/"))
          or is_allowed_disk_path($path)
          or ($path | startswith("/dev/mapper/"))
          or ($path | startswith("/dev/md/"))
          or ($path | test("^/dev/[A-Za-z0-9_.+-]+/[A-Za-z0-9_.+-]+$"))
          or ($path | startswith("/dev/zvol/"))
        | not
      )
    ' "$apply_json" | sort -u
  )"
  if [[ -n "$unsafe" ]]; then
    echo "refusing destructive plan with host path target(s) for $spec" >&2
    printf '%s\n' "$unsafe" >&2
    return 1
  fi
}

# shellcheck disable=SC2329
cleanup_on_exit() {
  if [[ "$execute" == "1" ]]; then
    cleanup_storage
  fi
  if [[ -n "$shim_dir" ]]; then
    rm -rf "$shim_dir"
  fi
  if [[ -n "$execute_specs_dir" ]]; then
    rm -rf "$execute_specs_dir"
  fi
}

trap cleanup_on_exit EXIT

if [[ ! -d "$examples_dir" ]]; then
  echo "examples directory not found: $examples_dir" >&2
  exit 1
fi

if [[ "$execute" == "1" ]]; then
  if [[ "$confirm" != "$required_confirm" ]]; then
    echo "refusing destructive run" >&2
    echo "set DISK_NIX_DISKO_E2E_CONFIRM=$required_confirm to wipe: $test_disk_list" >&2
    exit 1
  fi
  if [[ "$(id -u)" != "0" ]]; then
    echo "destructive E2E requires root" >&2
    exit 1
  fi
  for disk in "${test_disks[@]}"; do
    if [[ ! -b "$disk" ]]; then
      echo "required test disk is missing: $disk" >&2
      exit 1
    fi
    if lsblk -nr -o MOUNTPOINTS "$disk" | grep -q '[^[:space:]]'; then
      echo "refusing because $disk or a child has a mountpoint" >&2
      lsblk -o NAME,PATH,SIZE,TYPE,FSTYPE,MOUNTPOINTS "$disk" >&2
      exit 1
    fi
  done
  lsblk -o NAME,PATH,SIZE,TYPE,FSTYPE,MOUNTPOINTS "${test_disks[@]}"
  mkdir -p "$e2e_root"
  create_execute_shims
  if command -v modprobe >/dev/null; then
    modprobe md_mod 2>/dev/null || true
    modprobe zfs 2>/dev/null || true
    modprobe bcachefs 2>/dev/null || true
  fi
fi

if [[ "$execute" == "1" || "$preflight" == "1" ]]; then
  execute_specs_dir="$(mktemp -d)"
fi

fail=0
executed_specs=0
skipped_specs=0
skipped_reasons=()
for spec in "$examples_dir"/*.json; do
  [[ "$(basename "$spec")" == "manifest.json" ]] && continue
  spec_name="$(basename "$spec" .json)"
  run_spec="$spec"
  if [[ "$execute" == "1" || "$preflight" == "1" ]]; then
    run_spec="$execute_specs_dir/$(basename "$spec")"
    rewrite_spec_for_execute "$spec" "$run_spec" "$spec_name"
  fi
  if [[ "$execute" == "1" ]]; then
    if spec_requires_zfs "$run_spec" && ! zfs_is_available; then
      echo "== $(basename "$spec")"
      echo "skipping destructive execute because ZFS kernel support is unavailable"
      skipped_specs=$((skipped_specs + 1))
      skipped_reasons+=("$(basename "$spec"): missing ZFS kernel support")
      if [[ "$require_all_kernels" == "1" ]]; then
        fail=1
      fi
      continue
    fi
    if spec_requires_bcachefs "$run_spec" && ! bcachefs_is_available; then
      echo "== $(basename "$spec")"
      echo "skipping destructive execute because bcachefs kernel support is unavailable"
      skipped_specs=$((skipped_specs + 1))
      skipped_reasons+=("$(basename "$spec"): missing bcachefs kernel support")
      if [[ "$require_all_kernels" == "1" ]]; then
        fail=1
      fi
      continue
    fi
    cleanup_storage "$run_spec"
    wipe_test_disks
  fi
  echo "== $(basename "$spec")"

  plan_json="$(mktemp)"
  apply_json="$(mktemp)"
  if ! "$disk_nix_bin" plan --spec "$run_spec" --json >"$plan_json"; then
    echo "plan failed for $spec" >&2
    fail_current_spec "$run_spec"
    continue
  fi

  apply_args=(apply --spec "$run_spec" --json)
  if ! "$disk_nix_bin" "${apply_args[@]}" >"$apply_json"; then
    echo "apply failed for $spec" >&2
    cat "$apply_json" >&2 || true
    fail_current_spec "$run_spec"
    continue
  fi

  jq -r '"commands=\(.commandSummary.commandCount) ready=\(.commandSummary.readyCount) missingDomain=\(.commandSummary.needsDomainImplementationCount) manualOnly=\(.commandSummary.manualOnlyCount) blocked=\(.apply.blockedCount)"' "$apply_json"
  if jq -e '.apply.blockedCount != 0 or .commandSummary.needsDomainImplementationCount != 0 or .commandSummary.manualOnlyCount != 0 or .commandSummary.readyCount != .commandSummary.commandCount' "$apply_json" >/dev/null; then
    echo "non-ready command plan for $spec" >&2
    jq '.apply.blockedSummary, .commandSummary, [.commandPlan[] | {actionId, notReady: [.commands[] | select(.readiness != "ready")]} | select(.notReady|length>0)]' "$apply_json" >&2
    fail_current_spec "$run_spec"
    continue
  fi
  if [[ "$execute" == "1" || "$preflight" == "1" ]]; then
    if ! validate_execute_plan_paths "$apply_json" "$spec"; then
      fail_current_spec "$run_spec"
      continue
    fi
  fi
  if [[ "$execute" == "1" ]]; then
    execute_json="$(mktemp)"
    if ! "$disk_nix_bin" apply --spec "$run_spec" --json --execute >"$execute_json"; then
      echo "execute failed for $spec" >&2
      jq '.status, .partialExecutionRecovery, .messages, (.executionResults[]? | select(.success == false))' "$execute_json" >&2 || cat "$execute_json" >&2 || true
      fail_current_spec "$run_spec"
      continue
    fi
    executed_specs=$((executed_specs + 1))
  fi
  if [[ "$execute" == "1" ]]; then
    cleanup_storage "$run_spec"
  fi
done

if [[ "$execute" == "1" ]]; then
  echo "destructive execution summary: executed=$executed_specs skipped=$skipped_specs"
  if [[ "${#skipped_reasons[@]}" -gt 0 ]]; then
    printf 'skipped destructive example: %s\n' "${skipped_reasons[@]}"
  fi
  if [[ "$require_all_kernels" == "1" && "$skipped_specs" -gt 0 ]]; then
    echo "refusing success because DISK_NIX_DISKO_E2E_REQUIRE_ALL_KERNELS=1 and destructive examples were skipped" >&2
  fi
fi

exit "$fail"
