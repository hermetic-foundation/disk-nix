# Integration tests

Unit tests and flake checks cover parsers, planning, command rendering, NixOS
module evaluation, examples, schema generation, completions, and manpage output.
Real storage mutation needs additional host-backed tests because Nix build
sandboxes cannot safely create privileged block devices.

## VM destructive suite

The preferred destructive workflow is to run the smoke harnesses inside a
disposable virtual machine. The flake exposes an opt-in NixOS VM test that
boots a guest and runs the suite inside it:

```sh
nix build .#integration-vm-test
```

This derivation is intentionally not part of default `nix flake check`; it runs
QEMU and performs real storage mutations inside the guest.

If you already have a disposable VM or lab guest, run the in-guest suite
directly:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-vm-smoke
```

The VM suite refuses to run unless:

- `DISK_NIX_INTEGRATION_DESTRUCTIVE=1` is set
- it is running as root
- `systemd-detect-virt --vm` detects a virtual machine

For controlled lab automation where VM detection is unavailable but isolation
is provided externally, set `DISK_NIX_INTEGRATION_ASSUME_VM=1`.

By default the suite runs the loop, Btrfs, swap, layered-VM, and
failure-recovery smoke harnesses. To run a subset:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_VM_HARNESSES="loop btrfs" \
  nix run .#integration-vm-smoke
```

The individual harnesses below remain available for targeted lab debugging,
but they should still be treated as destructive host operations.

The ZFS harness is packaged with the VM suite and can be selected explicitly
with `DISK_NIX_VM_HARNESSES=zfs` in a disposable guest that has working ZFS
kernel support and a configured host ID. It is not part of the default VM suite
until the flake VM test provisions that kernel support reliably.

The NFS client harness is also packaged with the VM suite and can be selected
explicitly with `DISK_NIX_VM_HARNESSES=nfs` when the guest can reach a
disposable export supplied through `DISK_NIX_NFS_SOURCE`. It is not part of the
default VM suite because the flake VM test does not yet provision a server
export.

The VDO harness is packaged with the VM suite and can be selected explicitly
with `DISK_NIX_VM_HARNESSES=vdo` when the guest has an existing disposable VDO
volume named by `DISK_NIX_VDO_NAME`. It is not part of the default VM suite
because the flake VM test does not yet provision a VDO volume.

The iSCSI harness is packaged with the VM suite and can be selected explicitly
with `DISK_NIX_VM_HARNESSES=iscsi` when the guest has an existing disposable
iSCSI session for the target named by `DISK_NIX_ISCSI_TARGET`. It is not part
of the default VM suite because the flake VM test does not yet provision an
iSCSI target.

The multipath harness is packaged with the VM suite and can be selected
explicitly with `DISK_NIX_VM_HARNESSES=multipath` when the guest has an
existing disposable multipath map named by `DISK_NIX_MULTIPATH_MAP`. It is not
part of the default VM suite because the flake VM test does not yet provision
multiple backing paths for a map.

The NVMe harness is packaged with the VM suite and can be selected explicitly
with `DISK_NIX_VM_HARNESSES=nvme` when the guest has an existing disposable
controller path named by `DISK_NIX_NVME_CONTROLLER`. It is not part of the
default VM suite because the flake VM test does not yet provision an NVMe
controller.

## Failure-recovery smoke test

The repository includes a synthetic failed-apply harness:

```sh
env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-failure-recovery-smoke
```

The harness refuses to run unless `DISK_NIX_INTEGRATION_DESTRUCTIVE=1` is set,
matching the execute-mode integration guard used by the destructive harnesses.
It does not require root and does not mutate real storage. Instead, it uses
fake storage tools ahead of `PATH` for eighty-seven failed apply paths:

- a layered LVM volume grow followed by an ext4 filesystem grow where fake
  `lvextend` succeeds and fake `resize2fs` fails
- an LVM volume grow where fake `lvs --reportformat json vg0/root` succeeds
  and fake `lvextend --resizefs --size 50GiB vg0/root` fails
- an LVM thin-pool create where fake `vgs --reportformat json` succeeds and
  fake `lvcreate --type thin-pool --size 100GiB --name newpool vg0` fails
- an LVM thin-pool grow where fake thin-pool `lvs` inspection succeeds and
  fake `lvextend --size 500GiB vg0/thinpool` fails
- an XFS filesystem grow where fake `disk-nix inspect /` succeeds and fake
  `xfs_growfs /` fails
- a Btrfs scrub where fake `btrfs scrub start -B /data` fails
- a Btrfs rebalance where fake
  `btrfs balance start -dusage=50 -musage=75 /data` fails
- a Btrfs device replacement where fake `disk-nix inspect /data` succeeds and
  fake
  `btrfs replace start /dev/disk/by-id/old-btrfs-device /dev/disk/by-id/new-btrfs-device /data`
  fails
- a bcachefs device replacement where fake `bcachefs fs usage /bulk` and
  `bcachefs device add /bulk /dev/disk/by-id/new-bcachefs-device` succeed and
  fake `bcachefs data rereplicate /bulk` fails after one mutating command
- a filesystem trim where fake `disk-nix inspect /scratch` succeeds and fake
  `fstrim -v /scratch` fails
- an ext4 filesystem check where fake `disk-nix inspect /home` succeeds and
  fake `e2fsck -n /dev/disk/by-label/home` fails
- a Btrfs filesystem repair where fake target and mount-state inspections
  succeed and fake `btrfs check --repair /dev/disk/by-label/data` fails
- an XFS filesystem property mutation where fake `disk-nix inspect /scratch`
  succeeds and fake `xfs_admin -L scratch-new /dev/disk/by-label/scratch-old`
  fails
- a swap label mutation where fake
  `swaplabel --label swap-new /dev/disk/by-label/swap-old` fails
- a zram inventory rescan where fake
  `zramctl --bytes --raw --noheadings --output-all` fails before any mutating
  command runs
- a zram property reconciliation where fake
  `zramctl --bytes --raw --noheadings --output-all` fails while the generated
  `zram:set-property:algorithm` action remains pending for retry review
- a loop-device inventory rescan where fake
  `losetup --json --list /dev/loop7` fails before any mutating command runs
- a backing-file inventory rescan where fake
  `stat --printf=%n %s %b %B\\n /var/lib/images/inventory.img` fails before
  any mutating command runs
- a backing-file grow where fake `stat` succeeds and fake
  `truncate --size 16GiB /var/lib/images/root.img` fails
- a backing-file create where fake `test ! -e /var/lib/images/new.img`
  succeeds and fake `truncate --size 8GiB /var/lib/images/new.img` fails
- a device-mapper rename where fake `dmsetup info` and `dmsetup deps` succeed
  and fake `dmsetup rename /dev/mapper/cryptswap cryptswap-retired` fails
- a ZFS dataset rename where fake `zfs list -H -p tank/home` succeeds and fake
  `zfs rename tank/home tank/home-staged` fails
- a Btrfs snapshot clone where fake `btrfs subvolume show /mnt/persist/@home-before` succeeds and fake
  `btrfs subvolume snapshot -r /mnt/persist/@home-before /mnt/persist/@home-review`
  fails
- a ZFS snapshot clone where fake `zfs list -t snapshot -H -p tank/home@before` succeeds and fake
  `zfs clone tank/home@before tank/home-review` fails
- an LVM volume group rename where fake `vgs --reportformat json vg-old`
  succeeds and fake `vgrename vg-old vg-new` fails
- a ZFS pool device replacement where fake `disk-nix inspect tank` succeeds
  and fake
  `zpool replace tank /dev/disk/by-id/old-zfs-vdev /dev/disk/by-id/new-zfs-vdev`
  fails
- a ZFS snapshot rollback where fake `zfs list` succeeds and fake
  `zfs rollback tank/home@before` fails
- an NVMe namespace create where fake `nvme list-ns` succeeds and fake
  `nvme create-ns /dev/nvme0 --nsze-si 100G --ncap-si 100G` fails
- an NVMe namespace grow where fake namespace and subsystem inventory succeeds
  and fake `nvme ns-rescan /dev/nvme1` fails
- an NVMe namespace attach where fake namespace and subsystem inventory
  succeeds and fake `nvme attach-ns /dev/nvme2 --namespace-id 7 --controllers 0x2` fails
- an NVMe namespace detach where fake namespace and subsystem inventory
  succeeds and fake `nvme detach-ns /dev/nvme3 --namespace-id 8 --controllers 0x3` fails
- an NVMe namespace destroy where fake `nvme detach-ns` succeeds and fake
  `nvme delete-ns /dev/nvme4 --namespace-id 9` fails
- a target-side LUN create through the Linux LIO provider where fake
  `targetcli` inventory, backstore creation, and target creation succeed and
  fake `targetcli /iscsi/iqn.2026-06.example:storage.root/tpg1/luns create /backstores/block/_dev_zvol_tank_root lun=7`
  fails
- a target-side LUN attach through the Linux LIO provider where fake
  `targetcli` inventory and LUN mapping succeed and fake
  `targetcli /iscsi/iqn.2026-06.example:storage.root/tpg1/acls create iqn.2026-06.example:host.primary`
  fails
- a target-side LUN detach through the Linux LIO provider where fake
  `targetcli` inventory and ACL removal succeed and fake
  `targetcli /iscsi/iqn.2026-06.example:storage.root/tpg1/luns delete 7`
  fails after the reviewed potential-data-loss gate is enabled
- a target-side LUN destroy through the Linux LIO provider where fake
  `targetcli` inventory, ACL removal, LUN unmap, and target removal succeed and
  fake `targetcli /backstores/block delete _dev_zvol_tank_root` fails
- a target-side LUN grow plus property update through the Linux LIO provider
  where fake `targetcli`, `blockdev`, `lsscsi`, `multipath`, and `disk-nix`
  probes prove the native grow path validates backing capacity, persists target
  state, and verifies host-visible capacity
- a target-side LUN property update through the Linux LIO provider where fake
  target and backstore inventory succeeds and fake
  `targetcli /backstores/block/_dev_zvol_tank_root set attribute emulate_write_cache=0`
  fails
- a target-side LUN rescan through the Linux LIO provider where fake
  `targetcli /iscsi/iqn.2026-06.example:storage.root ls` inventory fails
- a target-side LUN create through the Linux tgt provider where fake `tgtadm`
  inventory and target creation succeed and fake
  `tgtadm --lld iscsi --mode logicalunit --op new --tid 42 --lun 8 --backing-store /dev/zvol/tank/root`
  fails
- a target-side LUN attach through the Linux tgt provider where fake `tgtadm`
  inventory and logical-unit mapping succeed and fake
  `tgtadm --lld iscsi --mode target --op bind --tid 42 --initiator-address ALL`
  fails
- a target-side LUN detach through the Linux tgt provider where fake `tgtadm`
  inventory and initiator unbind succeed and fake
  `tgtadm --lld iscsi --mode logicalunit --op delete --tid 42 --lun 8`
  fails after the reviewed potential-data-loss gate is enabled
- a target-side LUN destroy through the Linux tgt provider where fake `tgtadm`
  inventory, initiator unbind, and logical-unit delete succeed and fake
  `tgtadm --lld iscsi --mode target --op delete --tid 42` fails
- a target-side LUN grow plus property update through the Linux tgt provider
  where fake `tgtadm`, `blockdev`, `tgt-admin`, `lsscsi`, `multipath`, and
  `disk-nix` probes prove the native grow path validates backing capacity,
  refreshes the logical unit, captures persistent-config state, and verifies
  host-visible capacity
- a target-side LUN property update through the Linux tgt provider where fake
  target inventory succeeds and fake
  `tgtadm --lld iscsi --mode logicalunit --op update --tid 42 --lun 8 --name tgt.writeCache --value off`
  fails
- a target-side LUN rescan through the Linux tgt provider where fake
  `tgtadm --lld iscsi --mode target --op show --tid 42` inventory fails
- a target-side LUN create through the SCST provider where fake `scstadmin`
  inventory, backing-device open, target creation, initiator group creation,
  and initiator mapping succeed before fake `scstadmin -add_lun 9 ...` fails
- target-side LUN attach, detach, destroy, grow, property, and rescan flows
  through the SCST provider where fake `scstadmin` reaches the reviewed
  lifecycle command and fails with `partialExecutionRecovery`
- a host-side LUN rescan where fake `iscsiadm --mode session --rescan`,
  `lsscsi -t -s`, and `disk-nix inspect iqn.2026-06.example:storage/root:0`
  succeed before the reviewed `disk-nix-scsi-rescan` handoff fails
- a multipath map resize where fake `multipath -ll /dev/mapper/mpatha` and
  fake `lsscsi -t -s` succeed and fake
  `multipathd resize map /dev/mapper/mpatha` fails
- multipath path add, path remove, and host map flush flows where fake
  `multipath -ll /dev/mapper/mpatha` succeeds before fake `multipathd add`,
  fake `multipathd del`, or fake `multipath -f /dev/mapper/mpatha` fails
- a multipath path replacement where fake `multipath -ll /dev/mapper/mpatha`
  and fake `multipathd add path /dev/sdd` succeed and fake
  `multipathd del path /dev/sdc` fails
- an LVM volume group PV replacement where fake `pvs` inspections and
  `vgextend vg0 /dev/disk/by-id/new-pv` succeed and fake
  `pvmove /dev/disk/by-id/old-pv /dev/disk/by-id/new-pv` fails
- an MD RAID create where fake `/proc/mdstat` inspection succeeds and fake
  `mdadm --create /dev/md/newroot --level 1 --raid-devices 2 /dev/disk/by-id/nvme-a /dev/disk/by-id/nvme-b`
  fails after the reviewed destructive gate is enabled
- an MD RAID assemble where fake `/proc/mdstat` inspection succeeds and fake
  `mdadm --assemble /dev/md/existing /dev/disk/by-id/nvme-a /dev/disk/by-id/nvme-b`
  fails after the reviewed offline gate is enabled
- an MD RAID stop where fake `mdadm --detail /dev/md/oldroot` succeeds and
  fake `mdadm --stop /dev/md/oldroot` fails after the reviewed offline gate is
  enabled
- an MD RAID grow where fake `mdadm --detail /dev/md/root` succeeds and fake
  `mdadm --grow /dev/md/root --size max` fails
- an MD RAID member replacement where fake `mdadm --detail /dev/md/root`
  succeeds and fake
  `mdadm /dev/md/root --replace /dev/disk/by-id/old-md-member --with /dev/disk/by-id/new-md-member`
  fails
- an MD RAID member add where fake `mdadm --detail /dev/md/root` succeeds and
  fake `mdadm /dev/md/root --add /dev/disk/by-id/nvme-spare` fails
- a LUKS mapper open where fake `cryptsetup isLuks` succeeds and fake
  `cryptsetup open /dev/disk/by-id/archive-luks cryptarchive` fails
- a LUKS format where fake target inspection succeeds and fake
  `cryptsetup luksFormat /dev/disk/by-id/new-luks` fails after the reviewed
  destructive format gates are enabled
- a LUKS mapper close where fake `cryptsetup status cryptclosed` succeeds and
  fake `cryptsetup close cryptclosed` fails
- LUKS close execution semantics where a successful `cryptsetup close` followed
  by `cryptsetup status <mapper>` returning inactive status is accepted as the
  desired post-close verification result
- a LUKS grow where fake backing-device and mapper inspections succeed and fake
  `cryptsetup resize cryptroot` fails
- a LUKS keyslot add where fake `cryptsetup luksDump /dev/disk/by-id/root-luks`
  succeeds and fake
  `cryptsetup luksAddKey --key-slot 1 /dev/disk/by-id/root-luks /run/keys/root-new`
  fails
- a LUKS token import where fake `cryptsetup luksDump /dev/disk/by-id/root-luks`
  succeeds and fake
  `cryptsetup token import --token-id 0 --json-file /run/keys/root-token.json /dev/disk/by-id/root-luks`
  fails
- a LUKS keyslot removal where fake
  `cryptsetup luksDump /dev/disk/by-id/root-luks` succeeds and fake
  `cryptsetup luksKillSlot /dev/disk/by-id/root-luks 6` fails after the
  reviewed potential-data-loss gate is enabled
- a LUKS token removal where fake `cryptsetup luksDump /dev/disk/by-id/root-luks`
  succeeds and fake
  `cryptsetup token remove --token-id 9 /dev/disk/by-id/root-luks` fails after
  the reviewed potential-data-loss gate is enabled
- a LUKS header property mutation where fake `disk-nix inspect cryptroot`
  succeeds and fake
  `cryptsetup config /dev/disk/by-id/root-luks --label root-new` fails
- a partition grow where fake `disk-nix inspect /dev/disk/by-id/nvme-root-part2`
  succeeds and fake
  `parted -s /dev/disk/by-id/nvme-root resizepart 2 100%` fails
- an NFS remount where fake `findmnt --json /srv/tuned` succeeds and fake
  `mount -o remount,_netdev,ro,vers=4.2 /srv/tuned` fails
- an NFS unmount where fake `findmnt --json /srv/old` succeeds and fake
  `umount /srv/old` fails
- an NFS export where fake
  `exportfs -i -o rw,sync,no_subtree_check 192.0.2.0/24:/srv/share` fails
- an NFS unexport where fake `exportfs -u 192.0.2.55:/srv/old` fails
- an iSCSI session logout where fake `iscsiadm --logout` fails for the reviewed
  target and portal
- an iSCSI session login where fake `iscsiadm --mode discovery` succeeds and
  fake `iscsiadm --login` fails for the reviewed target and portal
- an iSCSI session rescan where fake `iscsiadm --mode session --rescan` fails
- an LVM cache attach where fake `lvconvert --type cache` fails for the
  reviewed origin LV and cache pool
- an LVM cache detach where fake `lvs` succeeds and fake
  `lvconvert --uncache` fails for the reviewed origin LV
- an LVM cache replacement where fake `disk-nix inspect vg0/root` succeeds and
  fake `lvconvert --uncache` plus replacement cache-pool attach handoff fails
  through the reviewed `disk-nix-lvm-cache-replace` shell wrapper
- an LVM cache rescan where fake `lvs --reportformat json` fails for the
  reviewed origin LV
- a VDO create where fake `disk-nix inspect /dev/disk/by-id/vdo-backing`
  succeeds and fake
  `vdo create --name new-cache --device /dev/disk/by-id/vdo-backing --vdoLogicalSize 2TiB`
  fails after the reviewed destructive gate is enabled
- a VDO rescan where fake `vdo status --name refreshArchive` succeeds and fake
  `vdostats --human-readable refreshArchive` fails
- a VDO logical grow where fake `vdo status --name archive` succeeds and fake
  `vdo growLogical --name archive --vdoLogicalSize 4TiB` fails
- a VDO physical grow where fake `vdo status --name archive-physical`
  succeeds and fake `vdo growPhysical --name archive-physical` fails after the
  reviewed backing-capacity check
- a VDO start where fake `vdo status --name warmArchive` succeeds and fake
  `vdo start --name warmArchive` fails
- a VDO stop where fake `vdo status --name coldArchive` succeeds and fake
  `vdo stop --name coldArchive` fails
- a VDO remove where fake `vdo status --name old-cache` succeeds and fake
  `vdo remove --name old-cache` fails after the reviewed destructive gate is
  enabled
- a VDO property mutation where fake `disk-nix inspect archive` succeeds and
  fake `vdo changeWritePolicy --name archive --writePolicy sync` fails
- a bcache cache replacement where fake `disk-nix inspect /dev/bcache0`
  succeeds and fake `make-bcache` plus sysfs detach/attach handoff fails
  through the reviewed `disk-nix-bcache-replace` shell wrapper
- a bcache property mutation where fake `disk-nix inspect /dev/bcache1`
  succeeds and fake sysfs `cache_mode` update fails for `/dev/bcache1`
- a bcache rescan where fake `disk-nix inspect /dev/bcache0` succeeds and fake
  sysfs `state` read fails for `/dev/bcache0`
- an LVM cache property mutation where fake `lvchange --cachemode` fails for a
  reviewed origin LV

The test verifies that the failed report and receipt preserve:

- `partialExecutionRecovery.completedActionIds` containing
  `volumes:vg0/root:grow`
- `partialExecutionRecovery.failedActionId` as `filesystem:root:grow`
- the failed `resize2fs vg0/root 50GiB` command and non-zero status
- one completed mutating command before failure
- `partialExecutionRecovery.failedActionId` as `volumes:root:grow`
- the failed `lvextend --resizefs --size 50GiB vg0/root` command and non-zero
  status after successful LV inspection
- `partialExecutionRecovery.failedActionId` as `thinpools:vg0/newpool:create`
- the failed `lvcreate --type thin-pool --size 100GiB --name newpool vg0`
  command and non-zero status after successful VG inspection
- `partialExecutionRecovery.failedActionId` as `thinpools:vg0/thinpool:grow`
- the failed `lvextend --size 500GiB vg0/thinpool` command and non-zero
  status after successful thin-pool inspection
- `partialExecutionRecovery.failedActionId` as `filesystem:root:grow`
- the failed `xfs_growfs /` command and non-zero status after successful
  filesystem target inspection
- `partialExecutionRecovery.failedActionId` as `filesystems:data:scrub`
- the failed `btrfs scrub start -B /data` command and non-zero status
- `partialExecutionRecovery.failedActionId` as `filesystems:data:rebalance`
- the failed `btrfs balance start -dusage=50 -musage=75 /data` command and
  non-zero status
- `partialExecutionRecovery.failedActionId` as `filesystems:scratch:trim`
- the failed `fstrim -v /scratch` command and non-zero status after successful
  filesystem target inspection
- `partialExecutionRecovery.failedActionId` as `filesystems:home:check`
- the failed `e2fsck -n /dev/disk/by-label/home` command and non-zero status
  after successful filesystem target inspection
- `partialExecutionRecovery.failedActionId` as `filesystems:data:repair`
- the failed `btrfs check --repair /dev/disk/by-label/data` command and
  non-zero status after target and mount-state inspection
- `partialExecutionRecovery.failedActionId` as
  `snapshot:tank/home@before:rollback`
- the failed `zfs rollback tank/home@before` command and non-zero status
- `partialExecutionRecovery.failedActionId` as
  `nvmenamespaces:/dev/nvme0:create`
- the failed `nvme create-ns /dev/nvme0 --nsze-si 100G --ncap-si 100G` command
  and non-zero status after namespace inventory inspection
- `partialExecutionRecovery.failedActionId` as
  `nvmenamespaces:/dev/nvme1:grow`
- the failed `nvme ns-rescan /dev/nvme1` command and non-zero status after
  namespace and subsystem inventory inspection
- `partialExecutionRecovery.failedActionId` as
  `nvmenamespaces:/dev/nvme2:attach`
- the failed `nvme attach-ns /dev/nvme2 --namespace-id 7 --controllers 0x2`
  command and non-zero status after namespace and subsystem inventory
  inspection
- `partialExecutionRecovery.failedActionId` as
  `nvmenamespaces:/dev/nvme3:detach`
- the failed `nvme detach-ns /dev/nvme3 --namespace-id 8 --controllers 0x3`
  command and non-zero status after namespace and subsystem inventory
  inspection
- `partialExecutionRecovery.failedActionId` as
  `nvmenamespaces:/dev/nvme4:destroy`
- the failed `nvme delete-ns /dev/nvme4 --namespace-id 9` command and
  non-zero status
- `partialExecutionRecovery.failedActionId` as
  `targetluns:iqn.2026-06.example:storage.root:create`
- the failed LIO `targetcli .../tpg1/luns create` command and non-zero status
  after target-side inventory, backstore creation, and target creation
- `partialExecutionRecovery.failedActionId` as
  `targetluns:iqn.2026-06.example:storage.root:attach`
- the failed LIO `targetcli .../tpg1/acls create` command and non-zero status
  after target-side inventory and LUN mapping
- `partialExecutionRecovery.failedActionId` as
  `targetluns:iqn.2026-06.example:storage.root:detach`
- the failed LIO `targetcli .../tpg1/luns delete` command and non-zero status
  after target-side inventory and ACL removal
- `partialExecutionRecovery.failedActionId` as
  `targetluns:iqn.2026-06.example:storage.root:destroy`
- the failed LIO `targetcli /backstores/block delete _dev_zvol_tank_root`
  command and non-zero status after target-side inventory, ACL removal, LUN
  unmap, and target removal
- a LIO target-side grow/property success report with concrete target/backstore
  inventory, backing-capacity validation, `saveconfig`, and host-visible
  verification results
- `partialExecutionRecovery.failedActionId` as
  `targetluns:iqn.2026-06.example:tgt.root:destroy`
- the failed Linux tgt `tgtadm --lld iscsi --mode target --op delete --tid 42`
  command and non-zero status after inventory, initiator unbind, and
  logical-unit delete
- `partialExecutionRecovery.failedActionId` as
  `targetluns:iqn.2026-06.example:tgt.root:attach`
- the failed Linux tgt `tgtadm --lld iscsi --mode target --op bind --tid 42 --initiator-address ALL`
  command and non-zero status after inventory and logical-unit mapping
- `partialExecutionRecovery.failedActionId` as
  `targetluns:iqn.2026-06.example:tgt.root:detach`
- the failed Linux tgt `tgtadm --lld iscsi --mode logicalunit --op delete --tid 42 --lun 8`
  command and non-zero status after inventory and initiator unbind
- a Linux tgt target-side grow/property success report with concrete target
  inventory, backing-capacity validation, logical-unit refresh, `tgt-admin --dump`, and host-visible verification results
- `partialExecutionRecovery.failedActionId` as
  `targetluns:iqn.2026-06.example:scst.root:create`
- the failed SCST `scstadmin -add_lun 9 ...` command and non-zero status after
  target inventory, backing-device open, target creation, initiator group
  creation, and initiator mapping
- SCST attach, detach, destroy, grow, property, and rescan failure assertions
  for concrete `scstadmin` `-add_lun`, `-rem_lun`, `-rem_target`,
  `-resync_dev`, and `-set_lun_attr` command rendering
- `partialExecutionRecovery.failedActionId` as
  `luks.devices:cryptclosed:close`
- the failed `cryptsetup close cryptclosed` command and non-zero status after
  mapper status inspection
- `partialExecutionRecovery.failedActionId` as `luks.devices:cryptnew:format`
- the failed `cryptsetup luksFormat /dev/disk/by-id/new-luks` command and
  non-zero status after target inspection
- `partialExecutionRecovery.failedActionId` as `luks.devices:cryptroot:grow`
- the failed `cryptsetup resize cryptroot` command and non-zero status after
  backing-device and mapper inspections
- `partialExecutionRecovery.failedActionId` as
  `lukskeyslots:cryptroot:1:add-key`
- the failed
  `cryptsetup luksAddKey --key-slot 1 /dev/disk/by-id/root-luks /run/keys/root-new`
  command and non-zero status after LUKS header inspection
- `partialExecutionRecovery.failedActionId` as
  `lukstokens:cryptroot:0:import-token`
- the failed
  `cryptsetup token import --token-id 0 --json-file /run/keys/root-token.json /dev/disk/by-id/root-luks`
  command and non-zero status after LUKS header inspection
- `partialExecutionRecovery.failedActionId` as
  `lukskeyslots:rootremove:remove-key`
- the failed `cryptsetup luksKillSlot /dev/disk/by-id/root-luks 6` command and
  non-zero status after LUKS header inspection
- `partialExecutionRecovery.failedActionId` as
  `lukstokens:rootremove:remove-token`
- the failed
  `cryptsetup token remove --token-id 9 /dev/disk/by-id/root-luks` command and
  non-zero status after LUKS header inspection
- `partialExecutionRecovery.failedActionId` as
  `multipathMaps:root-map:replace-device:/dev/sdc`
- the failed `multipathd del path /dev/sdc` command and non-zero status after
  successful multipath map inspection and replacement-path add
- `partialExecutionRecovery.failedActionId` as `mdraids:newroot:create`
- the failed
  `mdadm --create /dev/md/newroot --level 1 --raid-devices 2 /dev/disk/by-id/nvme-a /dev/disk/by-id/nvme-b`
  command and non-zero status after successful `/proc/mdstat` inspection
- `partialExecutionRecovery.failedActionId` as `mdraids:existing:assemble`
- the failed
  `mdadm --assemble /dev/md/existing /dev/disk/by-id/nvme-a /dev/disk/by-id/nvme-b`
  command and non-zero status after successful `/proc/mdstat` inspection
- `partialExecutionRecovery.failedActionId` as `mdraids:oldroot:stop`
- the failed `mdadm --stop /dev/md/oldroot` command and non-zero status after
  successful MD detail inspection
- `partialExecutionRecovery.failedActionId` as
  `mdRaids:root:add-device:/dev/disk/by-id/nvme-spare`
- the failed `mdadm /dev/md/root --add /dev/disk/by-id/nvme-spare` command and
  non-zero status after successful MD detail inspection
- `partialExecutionRecovery.failedActionId` as
  `mdRaids:root:remove-device:/dev/disk/by-id/failed-md-member`
- the failed `mdadm /dev/md/root --remove /dev/disk/by-id/failed-md-member`
  command and non-zero status after successful MD detail inspection and member
  fail marking
- `partialExecutionRecovery.failedActionId` as
  `nfs.mounts:/srv/old:unmount`
- the failed `umount /srv/old` command and non-zero status after NFS mount
  inspection
- `partialExecutionRecovery.failedActionId` as
  `iscsisessions:iqn.2026-06.example:storage.old:logout`
- the failed `iscsiadm --mode node --targetname iqn.2026-06.example:storage.old --portal 192.0.2.11:3260 --logout` command
  and non-zero status
- `partialExecutionRecovery.failedActionId` as
  `iscsisessions:iqn.2026-06.example:storage.root:login`
- the failed `iscsiadm --mode node --targetname iqn.2026-06.example:storage.root --portal 192.0.2.10:3260 --login` command
  and non-zero status after successful discovery
- `partialExecutionRecovery.failedActionId` as
  `iscsisessions:iqn.2026-06.example:storage.root:rescan`
- the failed `iscsiadm --mode session --rescan` command and non-zero status
  with generic recovery and recovery-point preservation actions
- `partialExecutionRecovery.failedActionId` as
  `luns:iqn.2026-06.example:storage/root:0:rescan`
- the failed `disk-nix-scsi-rescan` shell handoff for the reviewed host-visible
  LUN path after one completed mutating session rescan
- `partialExecutionRecovery.failedActionId` as
  `lvmCaches:vg0/root:add-device:vg0/root-cache`
- the failed `lvconvert --type cache --cachepool vg0/root-cache vg0/root`
  command and non-zero status
- `partialExecutionRecovery.failedActionId` as
  `lvmCaches:vg0/root:remove-device:vg0/root-cache`
- the failed `lvconvert --uncache vg0/root` command and non-zero status after
  successful cache-state inspection
- `partialExecutionRecovery.failedActionId` as
  `vdovolumes:new-cache:create`
- the failed
  `vdo create --name new-cache --device /dev/disk/by-id/vdo-backing --vdoLogicalSize 2TiB`
  command and non-zero status after backing-device inspection
- `partialExecutionRecovery.failedActionId` as
  `vdovolumes:refresharchive:rescan`
- the failed `vdostats --human-readable refreshArchive` command and non-zero
  status after successful VDO status inspection, with generic read-only recovery
  actions
- `partialExecutionRecovery.failedActionId` as
  `vdovolumes:archive-physical:grow`
- the failed `vdo growPhysical --name archive-physical` command and non-zero
  status after successful VDO status inspection
- `partialExecutionRecovery.failedActionId` as
  `vdoVolumes:archive:set-property:writePolicy`
- the failed `vdo changeWritePolicy --name archive --writePolicy sync` command
  and non-zero status after target inspection
- `partialExecutionRecovery.failedActionId` as
  `vdovolumes:old-cache:destroy`
- the failed `vdo remove --name old-cache` command and non-zero status after
  successful status inspection
- `partialExecutionRecovery.failedActionId` as `zram:rescan`
- the failed `zramctl --bytes --raw --noheadings --output-all` command and
  non-zero status before any mutating command runs, with generic read-only
  recovery actions
- `partialExecutionRecovery.failedActionId` as `zram:inspect`
- the failed `zramctl --bytes --raw --noheadings --output-all` command and
  non-zero status while `zram:set-property:algorithm` remains queued for retry
  review
- `partialExecutionRecovery.failedActionId` as
  `loopdevices:/dev/loop7:rescan`
- the failed `losetup --json --list /dev/loop7` command and non-zero status
  before any mutating command runs, with local mapping recovery guidance
- `partialExecutionRecovery.failedActionId` as
  `backingfiles:inventory:rescan`
- the failed `stat --printf=%n %s %b %B\\n /var/lib/images/inventory.img`
  command and non-zero status before any mutating command runs, with backing
  file and local mapping recovery guidance
- `partialExecutionRecovery.failedActionId` as `backingfiles:root:grow`
- the failed `truncate --size 16GiB /var/lib/images/root.img` command and
  non-zero status after successful backing-file metadata inspection, with
  recovery-point preservation guidance
- `partialExecutionRecovery.failedActionId` as `backingfiles:new:create`
- the failed `truncate --size 8GiB /var/lib/images/new.img` command and
  non-zero status after the absent-file precondition succeeds, with
  recovery-point preservation guidance
- `partialExecutionRecovery.failedActionId` as
  `lvmCaches:vg0/root:set-property:lvm.cache-mode`
- the failed `lvchange --cachemode writethrough vg0/root` command and non-zero
  status
- retry/review, roll-forward review, rollback review, snapshot-preservation,
  and domain-recovery guidance for the failed applies

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-failure-recovery-smoke.sh
```

## Loop-backed smoke test

The repository includes a root-only loop-backed smoke harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-loop-smoke
```

The harness refuses to run unless `DISK_NIX_INTEGRATION_DESTRUCTIVE=1` is set.
When enabled, it:

- creates a temporary 64 MiB backing file
- applies `backingFiles.<path>.properties.mode = "0600"` and verifies the
  rendered `chmod 0600 <path>` command changed the temporary backing file mode
- attaches it to the next available `/dev/loop*`
- applies `loopDevices.<loop>.properties."loop.read-only" = true`, verifies the
  rendered `blockdev --setro <loop>` command succeeded, then applies `false`
  and verifies `blockdev --setrw <loop>`
- formats the temporary loop device with ext4
- verifies `disk-nix inspect <loop> --json` can see the real loop node
- executes a safe `loopDevices.<loop>.operation = "rescan"` apply plan
- grows the temporary backing file, refreshes the loop device capacity, and
  executes an ext4 `resizePolicy = "grow-only"` apply plan
- executes an ext4 filesystem property apply plan that sets
  `filesystems.loopSmokeLabel.properties.label`
- verifies the rendered `e2label <loop> disknix-loop` command succeeded and the
  loop device reports the new label
- verifies the generated JSON report was written and all executed commands
  succeeded
- detaches the loop device and removes the backing file during cleanup

The test intentionally formats only the temporary backing file it creates. It
must still be treated as destructive because it uses real kernel loop devices
and filesystem tooling.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-loop-smoke.sh
```

## Btrfs loop-backed smoke test

The repository also includes a root-only Btrfs loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-btrfs-smoke
```

When enabled, it:

- creates a temporary 128 MiB backing file
- attaches it to the next available `/dev/loop*`
- formats the temporary loop device with Btrfs
- mounts the filesystem in the temporary directory
- verifies `disk-nix inspect <mountpoint> --json` sees Btrfs topology
- applies `filesystems.btrfsSmokeLabel.properties.label = "disknix-btrfs"`
  against the mounted Btrfs filesystem
- verifies the generated JSON report was written, the rendered
  `btrfs filesystem label <mountpoint> disknix-btrfs` command succeeded, and
  `btrfs filesystem label <mountpoint>` reports the new label
- executes a `filesystems.<name>.operation = "scrub"` apply plan
- verifies the generated JSON report was written and the rendered
  `btrfs scrub start -B <mountpoint>` command succeeded
- writes a sentinel file, applies a `filesystems.<name>.replaceDevices` plan
  from the original loop device to a second loop device, verifies the rendered
  `btrfs replace start <old-loop> <new-loop> <mountpoint>` command succeeded,
  confirms the replacement device appears in `btrfs filesystem show`, and
  checks the sentinel remains readable from the mounted filesystem
- unmounts, detaches both loop devices, and removes the backing files during
  cleanup

This test intentionally formats and mounts only the temporary backing files it
creates. It still requires destructive opt-in because it uses real loop, mount,
and Btrfs tooling, including a real filesystem device replacement.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-btrfs-smoke.sh
```

## bcachefs loop-backed smoke test

The repository also includes a root-only bcachefs loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-bcachefs-smoke
```

When enabled, it:

- creates a temporary 512 MiB backing file
- attaches it to the next available `/dev/loop*`
- formats the temporary loop device with bcachefs
- mounts the filesystem in the temporary directory
- verifies `disk-nix inspect <mountpoint> --json` sees bcachefs topology
- executes a `filesystems.<name>.operation = "scrub"` apply plan
- verifies the generated JSON report was written and the rendered
  `bcachefs scrub <mountpoint>` command succeeded
- writes a sentinel file, applies a `filesystems.<name>.replaceDevices` plan
  from the original loop device to a second loop device, verifies the rendered
  `bcachefs device add`, `bcachefs data rereplicate`, and
  `bcachefs device remove` commands succeeded, confirms replacement-device
  superblock metadata with `bcachefs show-super`, and checks the sentinel
  remains readable from the mounted filesystem
- unmounts, detaches both loop devices, and removes the backing files during
  cleanup

This test intentionally formats and mounts only the temporary backing files it
creates. It still requires destructive opt-in because it uses real loop, mount,
and bcachefs tooling, including real member replacement.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-bcachefs-smoke.sh
```

## bcache loop-backed smoke test

The repository also includes a root-only bcache property mutation harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-bcache-smoke
```

When enabled, it:

- creates temporary backing, cache, and replacement-cache image files
- attaches all three files to disposable `/dev/loop*` devices
- loads the `bcache` kernel module and initializes a real bcache backing/cache
  pair with `make-bcache`
- finds the generated `/dev/bcache*` device for the temporary backing loop
- applies `caches.bcacheSmoke.properties."bcache.cache-mode" = "writethrough"`
- verifies the rendered `disk-nix-bcache-property` sysfs write succeeded
- checks `/sys/block/<bcache>/bcache/cache_mode` reports `writethrough`
- derives the live cache-set UUID from `/sys/block/<bcache>/bcache/cache`
- applies `caches.bcacheSmoke.removeDevices = [ "<cache-set-uuid>" ]` and
  verifies the rendered `disk-nix-bcache-detach` sysfs write succeeded
- applies `caches.bcacheFailedAttach.addDevices = [ "<invalid-cache-set-uuid>" ]`
  while detached, verifies the rendered `disk-nix-bcache-attach` sysfs write
  fails, and checks the failed-attach recovery report includes
  partial-execution metadata, retry review, domain recovery, and roll-forward
  review
- applies `caches.bcacheSmoke.addDevices = [ "<cache-set-uuid>" ]`, verifies
  the rendered `disk-nix-bcache-attach` sysfs write succeeded, reapplies
  `bcache.cache-mode = "writethrough"`, and checks the cache mode again
- applies `caches.bcacheReplacement.replaceDevices` from the original cache
  loop to the replacement cache loop with the live `cacheSetUuid`, verifies the
  rendered `disk-nix-bcache-replace` wrapper succeeded, and confirms the
  generated bcache device remains readable after replacement
- executes `caches.bcacheSmoke.operation = "rescan"` against the same generated
  bcache device
- verifies the read-only rescan ran `disk-nix inspect <bcache>` and
  `disk-nix-bcache-read` checks for `state`, `cache_mode`, and `dirty_data`
- stops the generated bcache device, detaches the loops, and removes the
  backing files during cleanup

This test intentionally writes bcache metadata only to the temporary backing
files it creates. It is VM-callable through `DISK_NIX_VM_HARNESSES=bcache`, but
it is not in the default VM suite because bcache kernel support varies by
runner.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-bcache-smoke.sh
```

## LUKS loop-backed smoke test

The repository also includes a root-only LUKS loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-luks-smoke
```

When enabled, it:

- creates a temporary 64 MiB backing file and temporary keyfile
- attaches the file to the next available `/dev/loop*`
- formats the temporary loop device as a LUKS container
- opens it as a temporary `/dev/mapper/*` mapping
- verifies `disk-nix inspect <mapper> --json` sees the mapping
- applies `luks.devices.luksSmokeLabel.properties.label = "disknix-luks"`
  against the real LUKS backing device
- verifies the generated JSON report was written, the rendered
  `cryptsetup config <loop> --label disknix-luks` command succeeded, and
  `cryptsetup luksDump <loop>` reports the new header label
- executes a `luks.devices.<name>.operation = "close"` apply plan with
  `allowOffline = true`
- verifies the generated JSON report was written and the rendered
  `cryptsetup close <mapper>` command succeeded
- detaches the loop device and removes the backing file and key material during
  cleanup

This test intentionally formats only the temporary backing file it creates. It
still requires destructive opt-in because it uses real loop, device-mapper, and
LUKS tooling.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-luks-smoke.sh
```

## Swap loop-backed smoke test

The repository also includes a root-only swap loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-swap-smoke
```

When enabled, it:

- creates a temporary 64 MiB backing file
- attaches the file to the next available `/dev/loop*`
- formats the temporary loop device with a swap signature and initial label
- verifies `disk-nix inspect <loop> --json` sees the swap metadata
- applies `swaps.swapSmokeLabel.properties.label = "disknix-swap"` against
  the real loop-backed swap signature
- verifies the generated JSON report was written, the rendered
  `swaplabel --label disknix-swap <loop>` command succeeded, and
  `blkid -s LABEL -o value <loop>` reports the new label
- detaches the loop device and removes the backing file during cleanup

This test intentionally formats only the temporary backing file it creates. It
still requires destructive opt-in because it uses real loop and swap-signature
tooling.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-swap-smoke.sh
```

## zram property reconciliation smoke test

The repository also includes a root-only zram property reconciliation harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-zram-smoke
```

When enabled, it:

- applies `zram.properties.algorithm` and `zram.properties.priority`
  declarations
- verifies `zram:set-property:algorithm` and `zram:set-property:priority`
  render only read-only inventory commands
- verifies the rendered `zramctl --bytes --raw --noheadings --output-all`,
  `swapon --show --bytes --raw`, and `disk-nix zram` commands succeeded
- verifies the command-plan notes direct operators to
  `services.disk-nix.zram` and NixOS `zramSwap` reconciliation
- writes and compares the generated JSON report

This harness intentionally does not recreate active `/dev/zram*` devices.
Changing live zram algorithm, size, priority, or writeback settings is modeled
as generator reconciliation through NixOS service options because active zram
swap may require coordinated `swapoff` and device recreation.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-zram-smoke.sh
```

## LVM loop-backed smoke test

The repository also includes a root-only LVM loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-lvm-smoke
```

When enabled, it:

- creates a temporary 512 MiB backing file
- attaches it to the next available `/dev/loop*`
- creates a temporary LVM physical volume, volume group, logical volume, thin
  pool, thin volume, snapshot, cache pool, and cached origin volume
- formats the cached origin as ext4, mounts it, and writes a sentinel file
- verifies `disk-nix inspect <vg> --json` sees the volume group
- applies `lvmCaches.<vg/lv>.properties.lvm.cache-mode = "writethrough"`
  against the real cached origin logical volume
- verifies the generated JSON report was written, the rendered
  `lvchange --cachemode writethrough <vg/lv>` command succeeded, and
  `lvs -o cache_mode <vg/lv>` reports `writethrough`
- applies `lvmCaches.<vg/lv>.removeDevices = [ "<vg/cachepool>" ]`, verifies
  `lvconvert --uncache <vg/lv>` succeeds, and checks the origin is no longer
  cached
- applies `lvmCaches.<vg/lv>.addDevices = [ "<vg/cachepool>" ]`, verifies
  `lvconvert --type cache --cachepool <vg/cachepool> <vg/lv>` succeeds, and
  checks the cache mode is restored to `writethrough`
- creates a replacement cache pool, applies
  `lvmCaches.<vg/lv>.replaceDevices = { "<vg/cachepool>" = "<vg/cachepool_replacement>"; }`,
  verifies the rendered `disk-nix-lvm-cache-replace` wrapper runs
  `lvconvert --uncache <vg/lv>` before attaching the replacement cache pool
  with `lvconvert --type cache --cachepool`, and checks the sentinel again
  after replacement
- verifies the cached-origin ext4 cache sentinel survives the cache-mode
  mutation, cache detach, cache reattach, cache replacement, and LVM rescan
  plans
- executes `volumeGroups.<name>.operation = "rescan"`,
  `volumes.<vg/lv>.operation = "rescan"`,
  `thinPools.<vg/pool>.operation = "rescan"`, and
  `lvmSnapshots.<vg/snapshot>.operation = "rescan"` apply plans
- verifies the generated JSON report was written and the rendered
  `pvscan --cache`, `vgscan`, `vgchange --refresh <vg>`, and LVM `lvs`
  inventory commands succeeded
- unmounts the cached origin, removes the temporary volume group, wipes the
  physical volume metadata, detaches the loop device, and removes the backing
  file during cleanup

This test intentionally writes LVM metadata only to the temporary backing file
it creates. It still requires destructive opt-in because it uses real loop and
LVM tooling.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-lvm-smoke.sh
```

## MD RAID loop-backed smoke test

The repository also includes a root-only MD RAID loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-mdraid-smoke
```

When enabled, it:

- creates three temporary 64 MiB backing files
- attaches them to the next available `/dev/loop*` devices
- creates a temporary RAID1 MD array with `mdadm`
- verifies `disk-nix inspect <array> --json` sees the array
- executes an `mdRaids.<name>.operation = "rescan"` apply plan
- verifies the generated JSON report was written and the rendered
  `mdadm --detail`, `mdadm --detail --scan`, `mdadm --examine --scan`, and
  `/proc/mdstat` inventory commands succeeded
- applies an `mdRaids.<name>.replaceDevices` plan, verifies
  `mdadm <array> --replace <old-loop> --with <new-loop>` succeeds, waits for
  replacement completion with `mdadm --wait <array>`, and verifies the
  replacement member appears in `mdadm --detail`
- fails and removes one RAID1 member from the temporary array, using the
  replacement member to prove the degraded path after replacement
- verifies stale member metadata remains inspectable with
  `mdadm --examine <removed-loop>`
- verifies `disk-nix inspect <array> --json` still sees the degraded array and
  the degraded rescan apply succeeds
- applies an `mdRaids.<name>.removeDevices` plan for the already-removed
  member, verifies the real `mdadm` command fails, and checks the failed-detach recovery
  report includes partial-execution metadata, retry review, domain recovery,
  and roll-forward review
- applies an `mdRaids.<name>.addDevices` plan for a missing member path,
  verifies the real `mdadm <array> --add <missing-path>` command fails, and
  checks the failed-reattach recovery report includes partial-execution
  metadata, retry review, domain recovery, and roll-forward review
- stops the array, wipes member superblocks, detaches the loop devices, and
  removes backing files during cleanup

This test intentionally writes MD RAID metadata only to the temporary backing
files it creates. It still requires destructive opt-in because it uses real
loop and MD RAID tooling.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-mdraid-smoke.sh
```

## ZFS loop-backed smoke test

The repository also includes a root-only ZFS loop-backed harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-zfs-smoke
```

When enabled, it:

- creates a temporary 512 MiB backing file
- attaches it to the next available `/dev/loop*`
- creates a temporary ZFS pool mounted in the temporary directory
- verifies `disk-nix inspect <pool> --json` sees ZFS pool topology
- applies `pools.<name>.properties.autotrim = "on"` against the real
  loop-backed ZFS pool
- verifies the generated JSON report was written, the rendered
  `zpool set autotrim=on <pool>` command succeeded, and
  `zpool get -H -o value autotrim <pool>` reports `on`
- executes a `pools.<name>.operation = "scrub"` apply plan
- verifies the generated JSON report was written and the rendered
  `zpool scrub <pool>` command succeeded
- applies a `pools.<name>.replaceDevices` plan from the original loop vdev to a
  second loop vdev, verifies the rendered
  `zpool replace <pool> <old-loop> <new-loop>` command succeeded, confirms the
  replacement vdev appears in `zpool status -P`, and checks the mountpoint still
  remains active
- destroys the temporary pool, detaches both loop devices, and removes the
  backing files during cleanup

This test intentionally writes ZFS pool labels only to the temporary backing
files it creates. It still requires destructive opt-in because it uses real
loop and ZFS tooling, including a real pool-device replacement. The host or
guest must already have working ZFS kernel support; on NixOS this usually also
means a configured `networking.hostId`.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-zfs-smoke.sh
```

## NFS client smoke test

The repository also includes a root-only NFS client harness for lab exports:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_NFS_SOURCE=server.example.com:/srv/disk-nix-smoke \
  nix run .#integration-nfs-smoke
```

When enabled, it:

- creates a temporary mountpoint
- mounts the NFS source from `DISK_NIX_NFS_SOURCE`
- verifies `disk-nix inspect <mountpoint> --json` sees NFS topology
- executes an `nfs.mounts.<mountpoint>.operation = "rescan"` apply plan
- verifies the rendered `findmnt --json <mountpoint>` and
  `nfsstat -m <mountpoint>` commands succeeded
- executes an `nfs.mounts.<mountpoint>.operation = "remount"` apply plan
- verifies the rendered `mount -o remount,<options> <mountpoint>` command
  succeeded
- when `DISK_NIX_NFS_DATA_SURVIVAL=1` is set, writes
  `disk-nix-nfs-sentinel.txt` to the mounted export, injects a failed remount
  apply, verifies the partial-execution recovery report includes
  `resume-after-fix`, verifies the sentinel remains readable, reruns a clean
  remount apply, and verifies the sentinel remains readable after the resumed
  apply
- when `DISK_NIX_NFS_EXPORT_PROPERTY=1` is set, creates a temporary local
  export path, applies `exports.<path>.properties.options`, verifies the
  rendered `exportfs -i -o <options> <client>:<path>` command succeeded, and
  checks `exportfs -v` lists the temporary export
- unmounts the temporary client mount during cleanup

This test does not provision an NFS server or export. It requires a disposable
export provided by the operator because server behavior, export policy, network
reachability, NFS version, and authentication vary by lab. The default
filesystem type is `nfs4`, the default mount options are `vers=4.2`, and the
default remount options reuse the mount options. Override them with
`DISK_NIX_NFS_FSTYPE`, `DISK_NIX_NFS_MOUNT_OPTIONS`, and
`DISK_NIX_NFS_REMOUNT_OPTIONS`. For server-side export option testing, set
`DISK_NIX_NFS_EXPORT_PROPERTY=1`; the harness exports a temporary directory to
`DISK_NIX_NFS_EXPORT_CLIENT` with `DISK_NIX_NFS_EXPORT_OPTIONS`, then unexports
it during cleanup. The defaults are `127.0.0.1` and
`ro,sync,no_subtree_check`.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_NFS_SOURCE=server.example.com:/srv/disk-nix-smoke \
  DISK_NIX_NFS_EXPORT_PROPERTY=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-nfs-smoke.sh
```

## VDO smoke test

The repository also includes a root-only VDO harness for existing lab volumes:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_VDO_NAME=archive \
  nix run .#integration-vdo-smoke
```

When enabled, it:

- verifies `vdo status --name <name>` can read the selected VDO volume
- verifies `vdostats --human-readable <name>` can read runtime counters
- verifies `disk-nix inspect <name> --json` sees VDO topology
- applies `vdoVolumes.<name>.properties.writePolicy` against the selected
  disposable VDO volume
- verifies the generated JSON report was written, the rendered
  `vdo changeWritePolicy --name <name> --writePolicy <policy>` command
  succeeded, and `vdo status --name <name>` reports the requested write policy
- executes a `vdoVolumes.<name>.operation = "rescan"` apply plan
- verifies the rendered `vdo status --name <name>`,
  `vdostats --human-readable <name>`, and `disk-nix inspect <name>` commands
  succeeded
- verifies the generated JSON report was written

This test does not create, grow, start, stop, or remove a VDO volume. It still
requires destructive opt-in because it reads real VDO management state and
changes the selected volume's write policy. It is intended for disposable lab
hosts where the named volume can be safely probed and mutated. The default
write policy is `sync`; override it with `DISK_NIX_VDO_WRITE_POLICY=auto`,
`sync`, or `async`.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_VDO_NAME=archive \
  DISK_NIX_VDO_WRITE_POLICY=sync \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-vdo-smoke.sh
```

## iSCSI session smoke test

The repository also includes a root-only iSCSI harness for existing lab
sessions:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_ISCSI_TARGET=iqn.2026-06.example:storage.root \
  DISK_NIX_LUN_PATH=/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage.root-lun-0 \
  nix run .#integration-iscsi-smoke
```

When enabled, it:

- verifies `iscsiadm --mode session` reports the selected target
- verifies `lsscsi -t -s` can read host-visible transport inventory
- verifies `disk-nix inspect <target> --json` sees iSCSI topology
- executes an `iscsiSessions.<target>.operation = "rescan"` apply plan
- verifies the rendered `iscsiadm --mode session --rescan`,
  `lsscsi -t -s`, and `disk-nix inspect <target> --json` commands succeeded
- when `DISK_NIX_LUN_PATH` is set, executes
  `luns.<target>:0.operation = "rescan"` for that host-visible path
- verifies the rendered host-side `disk-nix-scsi-rescan` handoff and
  `multipath -r` commands succeeded for the selected LUN path
- when `DISK_NIX_LUN_DATA_SURVIVAL=1` and `DISK_NIX_LUN_MOUNTPOINT` are set,
  writes `disk-nix-iscsi-lun-sentinel.txt` to an already-mounted filesystem on
  that LUN, injects a failed host-side LUN rescan, verifies the
  partial-execution recovery report includes `resume-after-fix`, verifies the
  sentinel remains readable, reruns a clean LUN rescan apply, and verifies the
  sentinel remains readable after the resumed operation
- verifies the generated JSON report was written

This test does not discover, log in to, log out from, grow, attach, detach, or
remove an iSCSI target or LUN. It still requires destructive opt-in because it
performs a real session rescan and, when `DISK_NIX_LUN_PATH` is set, a real
host-side LUN rescan. It is intended for disposable lab hosts where the named
session and optional LUN path can be safely refreshed.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_ISCSI_TARGET=iqn.2026-06.example:storage.root \
  DISK_NIX_LUN_PATH=/dev/disk/by-path/ip-192.0.2.10:3260-iscsi-iqn.2026-06.example:storage.root-lun-0 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-iscsi-smoke.sh
```

## Multipath map smoke test

The repository also includes a root-only multipath harness for existing lab
maps:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_MULTIPATH_MAP=mpatha \
  DISK_NIX_MULTIPATH_RESIZE=1 \
  DISK_NIX_MULTIPATH_ADD_PATH=/dev/sdb \
  DISK_NIX_MULTIPATH_REMOVE_PATH=/dev/sdc \
  DISK_NIX_MULTIPATH_REPLACE_OLD_PATH=/dev/sde \
  DISK_NIX_MULTIPATH_REPLACE_NEW_PATH=/dev/sdf \
  DISK_NIX_MULTIPATH_FLUSH=1 \
  nix run .#integration-multipath-smoke
```

When enabled, it:

- verifies `multipath -ll <map>` can read the selected map
- verifies `lsscsi -t -s` can read host-visible transport inventory
- verifies `disk-nix inspect <map> --json` sees multipath topology
- executes a `multipathMaps.inventory.operation = "rescan"` apply plan with
  `target = <map>`
- verifies the rendered `multipath -ll <map>`, `lsscsi -t -s`, and
  `multipath -r` commands succeeded
- when `DISK_NIX_MULTIPATH_RESIZE=1` is set, executes
  `multipathMaps.resize.operation = "grow"` with `target = <map>`
- verifies the rendered `multipathd resize map <map>` and follow-up
  `multipath -r` commands succeeded
- when `DISK_NIX_MULTIPATH_ADD_PATH` or `DISK_NIX_MULTIPATH_REMOVE_PATH` is
  set, executes `multipathMaps.paths.addDevices` and/or
  `multipathMaps.paths.removeDevices` for the explicitly named paths
- verifies the rendered `multipathd add path <path>` and
  `multipathd del path <path>` commands succeeded for those selected paths
- when `DISK_NIX_MULTIPATH_REPLACE_OLD_PATH` and
  `DISK_NIX_MULTIPATH_REPLACE_NEW_PATH` are set, executes
  `multipathMaps.paths.replaceDevices` for the explicit path pair
- verifies the rendered `multipathd add path <new-path>` command succeeds
  before `multipathd del path <old-path>` succeeds
- when `DISK_NIX_MULTIPATH_FLUSH=1` is set, executes
  `multipathMaps.flush.destroy = true` with `allowDestructive = true` and
  `backupVerified = true`
- verifies the rendered `multipath -f <map>` command succeeded
- verifies the generated JSON report was written

This test requires destructive opt-in because `multipath -r` reloads live maps,
`DISK_NIX_MULTIPATH_RESIZE=1` asks multipathd to resize the selected map, and
the add/remove/replace/flush variables mutate explicitly selected paths or
maps. It is intended for disposable lab hosts where the named map and paths can
be safely refreshed, replaced, or removed. Use an `mpath*` name such as
`mpatha` or a `/dev/mapper/*` path.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_MULTIPATH_MAP=mpatha \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-multipath-smoke.sh
```

## NVMe namespace smoke test

The repository also includes a root-only NVMe harness for existing lab
controllers:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_NVME_CONTROLLER=/dev/nvme0 \
  nix run .#integration-nvme-smoke
```

When enabled, it:

- verifies `nvme list-ns <controller> --all --output-format=json` can read
  namespace inventory
- verifies `nvme list-subsys --output-format=json` can read subsystem paths
- verifies `disk-nix inspect <controller> --json` sees NVMe topology
- executes an `nvmeNamespaces.<controller>.operation = "rescan"` apply plan
- verifies the rendered `nvme list-ns`, `nvme list-subsys`, and
  `nvme ns-rescan <controller>` commands succeeded
- when `DISK_NIX_NVME_CREATE_DELETE=1` is set with
  `DISK_NIX_NVME_NAMESPACE_ID`, `DISK_NIX_NVME_NAMESPACE_SIZE`, and
  `DISK_NIX_NVME_CONTROLLERS`, applies an
  `nvmeNamespaces.<controller>.operation = "create"` plan, verifies
  `nvme create-ns <controller> --nsze-si <size> --ncap-si <size>`,
  `nvme attach-ns <controller> --namespace-id <id> --controllers <ids>`, and
  namespace rescan succeed, then applies a destructive cleanup plan and verifies
  `nvme detach-ns <controller> --namespace-id <id> --controllers <ids>`,
  `nvme delete-ns <controller> --namespace-id <id>`, and final namespace
  inventory succeed
- verifies namespace identity drift for that create/delete path by checking
  `nvme list-ns <controller> --all --output-format=json` contains the selected
  namespace id after create and no longer contains it after delete
- when `DISK_NIX_NVME_GROW=1` is set, applies an
  `nvmeNamespaces.<controller>.operation = "grow"` plan and verifies the
  rendered `nvme list-subsys` and `nvme ns-rescan <controller>` commands
  succeeded under the reviewed grow policy
- when `DISK_NIX_NVME_ATTACH_DETACH=1` is set with
  `DISK_NIX_NVME_NAMESPACE_ID` and `DISK_NIX_NVME_CONTROLLERS`, applies an
  `nvmeNamespaces.<controller>.operation = "attach"` plan, verifies
  `nvme attach-ns <controller> --namespace-id <id> --controllers <ids>` and
  `nvme ns-rescan <controller>` succeed, then applies a matching detach plan
  and verifies `nvme detach-ns <controller> --namespace-id <id> --controllers <ids>` plus a final namespace rescan succeed
- verifies the generated JSON report was written

By default this test does not create, grow, attach, detach, or delete NVMe
namespaces. The create/delete and attach/detach modes are deliberately opt-in
and require a disposable namespace that can safely end deleted or detached from
the selected controller. The harness still requires destructive opt-in because
`nvme ns-rescan` refreshes live controller namespace state and namespace
lifecycle changes visibility or allocation. Use a controller path such as
`/dev/nvme0`, not a namespace path such as `/dev/nvme0n1`.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_NVME_CONTROLLER=/dev/nvme0 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-nvme-smoke.sh
```

## Target-side LUN property smoke test

The repository also includes a root-only LIO target-side LUN property harness:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-target-lun-smoke
```

When enabled, it:

- creates a temporary 64 MiB backing file and attaches it to a disposable
  `/dev/loop*`
- creates a temporary LIO block backstore with
  `targetcli /backstores/block create`
- creates a temporary iSCSI target and maps the first backstore as a LUN
- applies `targetLuns.<iqn>.properties."lio.writeCache" = "off"`
- verifies the rendered
  `targetcli /backstores/block/<name> set attribute emulate_write_cache=0`
  command succeeded
- creates a second temporary backstore and applies
  `targetLuns.<iqn>.operation = "attach"` to map it as another LUN with a
  reviewed initiator ACL
- applies `targetLuns.<iqn>.operation = "detach"` to remove that initiator ACL
  and unmap the second LUN without deleting the backstore
- verifies `targetLuns.<iqn>.destroy = true` is refused without
  `allowDestructive = true`, leaves the command plan empty, and reports
  non-destructive review-policy guidance
- removes the temporary target, backstores, loop devices, and backing files
  during cleanup

This test intentionally mutates only the temporary LIO target state and
loop-backed block device it creates. It is VM-callable with
`DISK_NIX_VM_HARNESSES=target-lun`, but it is not part of the default VM suite
because LIO kernel target support varies by runner.

To test a development build without `nix run`, set `DISK_NIX_BIN`:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  DISK_NIX_BIN=target/debug/disk-nix \
  ./scripts/integration-target-lun-smoke.sh
```

## Layered VM smoke test

The repository includes a root-only layered harness intended for disposable VMs:

```sh
sudo env DISK_NIX_INTEGRATION_DESTRUCTIVE=1 \
  nix run .#integration-layered-vm-smoke
```

When enabled, it:

- creates a temporary partitioned loop-backed disk image
- formats and opens a LUKS mapper on the loop partition
- creates an LVM PV, VG, and root LV on the mapper
- creates and mounts an ext4 filesystem on the LV
- verifies `disk-nix inspect <mountpoint> --json` sees the layered topology
- writes a sentinel file, grows the loop backing file, and executes one
  multi-domain apply plan for `partitions.layeredPart`,
  `luks.devices.layeredMapper`, `volumes.layeredRoot`,
  `filesystems.layeredRoot`, and `filesystems.layeredRootRemount`
- verifies the rendered and executed `growpart <loop> 1`,
  `cryptsetup resize <mapper>`, `lvextend --resizefs --size 192M <lv>`,
  `resize2fs <lv>`, and `mount -o remount,rw,noatime <mountpoint>` commands
  succeeded and the JSON report was written
- verifies the LV grew, the remount option is active, the sentinel survived, and
  the mounted filesystem remains inspectable after the grow
- executes a VM-backed failure-injection apply where real
  `lvextend --resizefs --size 256M <lv>` succeeds and real
  `xfs_growfs <mountpoint>` fails against the ext4 mount
- verifies the failed report records `partialExecutionRecovery` with the
  completed LV grow action, failed filesystem action, failed command, remaining
  remount action, completed mutating command count, fresh-topology review notes,
  and domain, roll-forward, rollback, and recovery-point preservation actions
- verifies rollback review stays non-mutating: rollback precondition commands
  are read-only, the rollback recipe is `refused`, reversible and destructive
  mutation sections are empty, required topology evidence is listed, and
  operator-only guidance is emitted instead of an automated unsafe rollback
- resumes with a clean follow-up apply for the remaining remount action,
  verifies `mount -o remount,rw,relatime <mountpoint>` succeeds, and confirms the
  sentinel remains readable after the failed-and-resumed apply sequence
- verifies the failed apply report was written, the LV growth before the failure
  is visible, and sentinel data still survives after the failed apply
- unmounts the filesystem, deactivates the VG, executes a
  `luks.devices.layeredMapper` close apply plan, and verifies the rendered
  `cryptsetup close <mapper>` command succeeded
- reopens the LUKS mapper with the temporary key, reactivates the VG, remounts
  the LV, verifies the sentinel survived, and inspects the reopened layered
  topology

The harness removes the mount, VG, mapper, loop device, backing file, and key
material during cleanup. It is included in the default VM smoke suite alongside
the loop, Btrfs, and synthetic failure-recovery harnesses, but it is not run by
`nix flake check` because it mutates real kernel block-device state. The LUKS,
LVM, MD RAID, bcachefs, ZFS, NFS, VDO, iSCSI, multipath, and NVMe harnesses
remain packaged and VM-callable through `DISK_NIX_VM_HARNESSES`; bcachefs is
not part of the default VM list because some NixOS test kernels do not expose
the `bcachefs` filesystem module even when `bcachefs-tools` is available.

## Flake coverage

`nix flake check` does not run destructive integration tests. It does validate
that the loop smoke harnesses parse, remain opt-in, and still contain the
expected loop, filesystem setup, resize, mount, Btrfs scrub, bcachefs format,
bcachefs scrub, LUKS format, LUKS open, LUKS close, LVM create, LVM rescan, MD
RAID create, MD RAID rescan, ZFS pool create, ZFS scrub, NFS mount, NFS rescan,
NFS remount, NFS export, NFS unexport, VDO status, VDO stats, VDO rescan, VM
orchestration guard steps, iSCSI session rescan, multipath map rescan, NVMe
namespace rescan, layered loop/LUKS/LVM/ext4 VM grow assertions, and the synthetic failed-apply
`partialExecutionRecovery` assertions. This keeps the harnesses available and
packaged while preserving safe default checks.

## Remaining integration coverage

The VM smoke suite and targeted loop tests are only the first host-backed
integration paths. Feature completion still needs disposable VM or lab-host
tests for broader LUKS format/grow/keyslot/token behavior, broader LVM
LV/thin/cache/device-topology behavior, additional device replacement domains,
broader bcachefs multi-device and member-topology behavior, broader ZFS
vdev/dataset/zvol/snapshot behavior, broader MD RAID grow/member-topology and
degraded-array variant behavior, broader multipath path
add/remove/flush/grow/failure behavior,
broader iSCSI LUN failure behavior, broader NFS server/client variant failure
behavior, broader VDO create/rescan/logical-grow/physical-grow/start/stop/property/remove behavior,
additional NVMe namespace variant failure behavior, additional cache variant
failure behavior, property mutation across more supported domains, recovery
behavior beyond the synthetic LVM-plus-filesystem, LVM grow, LVM thin-pool create, LVM thin-pool grow, XFS grow, Btrfs
scrub, Btrfs rebalance, Btrfs device replacement, bcachefs replacement,
filesystem trim, filesystem check, filesystem repair, filesystem property,
swap label, zram rescan, zram property inventory, loop rescan, backing-file rescan, backing-file grow, backing-file create, device-mapper rename, ZFS dataset rename, Btrfs snapshot clone,
ZFS snapshot clone, LVM VG rename, LVM VG replacement, ZFS pool replacement,
ZFS rollback, NVMe namespace create, NVMe namespace grow, NVMe
namespace attach, NVMe namespace detach, NVMe namespace delete, target-side LUN
LIO create, target-side LUN LIO attach, target-side LUN LIO detach,
target-side LUN LIO destroy, target-side LUN LIO native grow with backing
capacity and host verification, target-side LUN LIO property, target-side LUN LIO rescan,
target-side LUN tgt create, target-side LUN tgt attach, target-side LUN tgt
detach, target-side LUN tgt destroy, target-side LUN tgt native grow with
backing capacity and host verification, target-side LUN tgt property, target-side LUN tgt
rescan, target-side LUN SCST create, target-side LUN SCST attach,
target-side LUN SCST detach, target-side LUN SCST destroy, target-side LUN SCST
grow, target-side LUN SCST property, target-side LUN SCST rescan, host-side LUN rescan, multipath
resize, multipath add, multipath remove, multipath flush, multipath replace, MD RAID create, MD RAID assemble, MD RAID stop, MD RAID grow, MD RAID add-member, MD RAID remove-member, MD RAID replace, LUKS open, LUKS close, LUKS
format, LUKS grow, LUKS keyslot add, LUKS token import, LUKS keyslot remove,
LUKS token remove, LUKS property, partition grow, NFS
remount, NFS unmount, NFS export, NFS unexport, iSCSI logout, iSCSI login,
iSCSI rescan, LVM cache attach, LVM cache detach, LVM cache replacement, LVM cache rescan, VDO
create, VDO rescan, VDO logical grow, VDO physical grow, VDO start, VDO stop,
VDO remove, VDO property, bcache replacement, bcache property, bcache rescan,
filesystem property, and LVM cache property failed-command
paths, and broader destructive apply behavior.
