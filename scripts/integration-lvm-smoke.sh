#!/usr/bin/env bash
set -euo pipefail

if [[ "${DISK_NIX_INTEGRATION_DESTRUCTIVE:-}" != "1" ]]; then
  cat >&2 <<'MSG'
Refusing to run LVM loop-backed integration smoke test.

Set DISK_NIX_INTEGRATION_DESTRUCTIVE=1 to acknowledge that this test creates,
rescans, removes, and wipes a temporary loop-backed LVM physical volume and
volume group. The backing file is created in a temporary directory and removed
during cleanup.
MSG
  exit 2
fi

if [[ "$(id -u)" != "0" ]]; then
  echo "LVM loop-backed integration smoke test must run as root" >&2
  exit 2
fi

disk_nix_bin="${DISK_NIX_BIN:-disk-nix}"

for tool in "$disk_nix_bin" jq losetup lvs pvcreate pvremove pvscan pvs truncate vgchange vgcreate vgremove vgscan vgs; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required tool is missing: $tool" >&2
    exit 2
  fi
done

tmpdir="$(mktemp -d)"
loopdev=""
vg="disk_nix_lvm_smoke_$$"

cleanup() {
  if vgs "$vg" >/dev/null 2>&1; then
    vgchange --activate n "$vg" >/dev/null 2>&1 || true
    vgremove --force --force --yes "$vg" >/dev/null 2>&1 || true
  fi
  if [[ -n "$loopdev" ]]; then
    pvremove --force --force --yes "$loopdev" >/dev/null 2>&1 || true
  fi
  if [[ -n "$loopdev" ]] && losetup --list "$loopdev" >/dev/null 2>&1; then
    losetup --detach "$loopdev"
  fi
  rm -rf "$tmpdir"
}
trap cleanup EXIT

backing="$tmpdir/disk-nix-lvm-smoke.img"
spec="$tmpdir/spec.json"
report="$tmpdir/apply-report.json"

truncate --size 128M "$backing"
loopdev="$(losetup --find --show "$backing")"
pvcreate --force --yes "$loopdev"
vgcreate "$vg" "$loopdev"

"$disk_nix_bin" inspect "$vg" --json > "$tmpdir/inspect.json"
jq -e --arg vg "$vg" '
  (.matchedNodes // .nodes // [])
  | any(
      .name == $vg
      or .id == ("lvm-vg:" + $vg)
      or (.properties // [] | any(.key == "lvm.vg-name" and .value == $vg))
    )
' "$tmpdir/inspect.json" >/dev/null

jq -n --arg vg "$vg" '{
  version: 1,
  volumeGroups: {
    ($vg): {
      operation: "rescan"
    }
  }
}' > "$spec"

"$disk_nix_bin" apply \
  --spec "$spec" \
  --execute \
  --report-out "$report" \
  --json > "$tmpdir/apply.json"

jq -e --arg vg "$vg" '
  .status == "succeeded"
  and (.commandPlan[] | select(.actionId == ("volumegroups:" + $vg + ":rescan"))
    | .commands
    | any(.argv == ["pvscan", "--cache"])
    and any(.argv == ["vgscan"])
    and any(.argv == ["vgchange", "--refresh", $vg]))
  and (.executionResults
    | any(.argv == ["vgchange", "--refresh", $vg] and .success == true))
' "$tmpdir/apply.json" >/dev/null

cmp "$tmpdir/apply.json" "$report" >/dev/null
vgs --reportformat json "$vg" >/dev/null
pvs --reportformat json "$loopdev" >/dev/null

echo "LVM loop-backed integration smoke test refreshed $vg on $loopdev"
