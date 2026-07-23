fn lifecycle_verification(
    action: &PlannedAction,
    ctx: VerificationContext<'_>,
) -> Option<VerificationResult> {
    let VerificationContext {
        collection,
        target,
        cache_target,
        mountpoint,
        fs_type,
        desired_size,
    } = ctx;
    let _ = (cache_target, mountpoint, fs_type, desired_size);
    Some(match action.operation {
        Operation::Create | Operation::Open if collection == Some("luks.devices") => (
            vec![
                command(
                    ["cryptsetup", "status", target],
                    false,
                    "verify the LUKS mapper is open",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify opened mapper identity and graph relationships",
                ),
            ],
            vec![
                "mapper name and backing device match the desired declaration".to_string(),
                "dependent storage layers see the opened mapper path".to_string(),
            ],
        ),
        Operation::Create => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify the created storage object is present in the graph",
            )],
            vec![
                "created object identity, size, and relationships match desired state".to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("zvols") => (
            vec![command(
                ["zfs", "list", "-H", "-p", "-t", "volume"],
                false,
                "verify zvol inventory after destruction",
            )],
            vec![
                "destroyed zvol no longer appears in ZFS volume listings".to_string(),
                "downstream LUN, guest, or filesystem consumers are detached or updated"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("datasets") => (
            vec![command(
                ["zfs", "list", "-H", "-p", "-t", "filesystem"],
                false,
                "verify ZFS dataset inventory after destruction",
            )],
            vec![
                "destroyed dataset no longer appears in ZFS filesystem listings".to_string(),
                "mounts, descendants, snapshots, and consumers were drained or updated".to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("lvmSnapshots") => (
            vec![command(
                ["lvs", "--reportformat", "json"],
                false,
                "verify LVM snapshot inventory after removal",
            )],
            vec![
                "removed snapshot no longer appears in LVM reports".to_string(),
                "origin logical volume remains active and healthy".to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("volumes") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json"],
                    false,
                    "verify logical volume no longer appears in LVM inventory",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after logical volume removal",
                ),
            ],
            vec![
                "removed logical volume is absent from LVM reports".to_string(),
                "dependent filesystems, mappings, and mounts no longer reference the LV"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("thinPools") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json"],
                    false,
                    "verify thin pool no longer appears in LVM inventory",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after thin pool removal",
                ),
            ],
            vec![
                "removed thin pool is absent from LVM reports".to_string(),
                "dependent thin volumes, filesystems, mappings, and mounts are removed or migrated"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("volumeGroups") => (
            vec![
                command(
                    ["vgs", "--reportformat", "json"],
                    false,
                    "verify LVM volume group inventory after removal",
                ),
                command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "verify physical volume state after volume group removal",
                ),
            ],
            vec![
                "removed volume group no longer appears in LVM reports".to_string(),
                "no logical volumes or device-mapper nodes still depend on the removed VG"
                    .to_string(),
            ],
        ),
        Operation::Import | Operation::Export if collection == Some("volumeGroups") => (
            vec![
                command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    "verify LVM volume group inventory after import or export",
                ),
                command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "verify physical volume membership after VG import or export",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled VG graph relationships after import or export",
                ),
            ],
            vec![
                "volume group import or export state matches the declared lifecycle operation"
                    .to_string(),
                "logical volumes, filesystems, mappings, mounts, and services are reviewed after the VG state change"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status"],
                    false,
                    "verify VDO volume inventory after removal",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after VDO volume removal",
                ),
            ],
            vec![
                "removed VDO volume no longer appears in VDO status output".to_string(),
                "dependent filesystems, mappings, and mounts no longer reference the VDO device"
                    .to_string(),
            ],
        ),
        Operation::Stop if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status"],
                    false,
                    "verify VDO volume inventory after stop",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after VDO volume stop",
                ),
            ],
            vec![
                "stopped VDO volume is no longer active in VDO status output".to_string(),
                "dependent filesystems, mappings, and mounts no longer reference the stopped VDO device"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("pools") => (
            vec![
                command(
                    ["zpool", "list", "-H", "-p"],
                    false,
                    "verify ZFS pool inventory after destruction",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after pool destruction",
                ),
            ],
            vec![
                "destroyed pool no longer appears in ZFS pool listings".to_string(),
                "datasets, zvols, exports, and mounts have been migrated or removed".to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("loopDevices") => (
            vec![command(
                ["losetup", "--json", "--list"],
                false,
                "verify loop device is detached while backing file remains",
            )],
            vec![
                "loop device no longer appears in losetup inventory".to_string(),
                "backing file or block device remains intact".to_string(),
            ],
        ),
        Operation::Rollback if collection == Some("lvmSnapshots") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify LVM snapshot merge state",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify origin and snapshot graph state after rollback",
                ),
            ],
            vec![
                "snapshot merge is complete or queued for next activation".to_string(),
                "origin logical volume contents and consumers are verified after merge".to_string(),
            ],
        ),
        Operation::Rollback if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            if !is_zfs_snapshot_name(snapshot) {
                return Some((Vec::new(), Vec::new()));
            }
            let dataset = zfs_snapshot_dataset(snapshot).unwrap_or("<dataset>");
            (
                vec![
                    command(
                        ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                        false,
                        "verify the ZFS snapshot still exists after rollback",
                    ),
                    command(
                        ["zfs", "list", "-H", "-p", dataset],
                        false,
                        "verify the rolled-back ZFS dataset after rollback",
                    ),
                    command(
                        ["disk-nix", "inspect", dataset, "--json"],
                        false,
                        "verify dataset graph state and consumers after rollback",
                    ),
                ],
                vec![
                    "dataset contents match the reviewed snapshot rollback point".to_string(),
                    "newer snapshots, clones, mounts, and dependent services were reviewed after rollback"
                        .to_string(),
                ],
            )
        }
        Operation::Clone if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            let clone_target = action.context.target.as_deref().unwrap_or("<clone-dataset>");
            if is_zfs_snapshot_name(snapshot) {
                (
                    vec![
                        command(
                            ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                            false,
                            "verify source ZFS snapshot exists before clone",
                        ),
                        command(
                            ["zfs", "list", "-H", "-p", clone_target],
                            false,
                            "verify cloned ZFS dataset after clone",
                        ),
                        command(
                            ["disk-nix", "inspect", clone_target, "--json"],
                            false,
                            "verify cloned dataset graph state after clone",
                        ),
                    ],
                    vec![
                        "clone dataset exists and is mounted or configured as expected".to_string(),
                        "clone origin points at the reviewed source snapshot".to_string(),
                    ],
                )
            } else if is_btrfs_snapshot_pair(snapshot, clone_target) {
                (
                    vec![
                        command(
                            ["btrfs", "subvolume", "show", snapshot],
                            false,
                            "verify source Btrfs snapshot subvolume exists before clone",
                        ),
                        command(
                            ["btrfs", "subvolume", "show", clone_target],
                            false,
                            "verify cloned Btrfs subvolume after clone",
                        ),
                        command(
                            ["disk-nix", "inspect", clone_target, "--json"],
                            false,
                            "verify cloned Btrfs subvolume graph state after clone",
                        ),
                    ],
                    vec![
                        "clone subvolume exists at the reviewed path".to_string(),
                        "snapshot lineage and read-only state were reviewed after clone".to_string(),
                    ],
                )
            } else {
                (Vec::new(), Vec::new())
            }
        }
        Operation::Promote if collection == Some("datasets") || collection == Some("zvols") => {
            let target = action.context.target.as_deref().unwrap_or(target);
            (
                vec![
                    command(
                        ["zfs", "get", "-H", "-o", "value", "origin", target],
                        false,
                        "verify clone origin after promotion",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify promoted ZFS object graph state after promotion",
                    ),
                ],
                vec![
                    "promoted clone remains available at the reviewed dataset or zvol name"
                        .to_string(),
                    "origin dependency and dependent snapshots were reviewed after promotion"
                        .to_string(),
                ],
            )
        }
        Operation::Import | Operation::Export if collection == Some("pools") => {
            let target = zfs_pool_command_target(action, Some(target));
            (
                vec![
                    command(
                        ["zpool", "status", "-P", target],
                        false,
                        "verify ZFS pool health after import or export",
                    ),
                    command(
                        ["zpool", "list", "-H", "-p"],
                        false,
                        "verify active ZFS pool inventory after import or export",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify modeled pool graph relationships after import or export",
                    ),
                ],
                vec![
                    "pool import or export state matches the declared lifecycle operation"
                        .to_string(),
                    "datasets, mountpoints, shares, LUN mappings, and services are reviewed after the pool state change"
                        .to_string(),
                ],
            )
        }
        Operation::Format if collection == Some("swaps") => (
            vec![
                command(
                    ["blkid", target],
                    false,
                    "verify swap signature identity after mkswap",
                ),
                command(
                    ["swapon", "--show", "--bytes", "--raw"],
                    false,
                    "verify swap activation state after signature creation",
                ),
            ],
            vec![
                "target has a swap signature and no unexpected filesystem signature".to_string(),
                "swap activation follows the desired NixOS swapDevices configuration".to_string(),
            ],
        ),
        Operation::Format if collection == Some("luks.devices") => (
            vec![
                command(
                    ["cryptsetup", "isLuks", target],
                    false,
                    "verify the target is a LUKS container",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify encrypted container identity and graph relationships",
                ),
            ],
            vec![
                "LUKS header exists and recovery header backup has been captured".to_string(),
                "mapper name and backing device match the desired declaration".to_string(),
            ],
        ),
        Operation::Format if collection == Some("filesystems") => {
            let device = action.context.device.as_deref().unwrap_or(target);
            (
                vec![
                    command(
                        ["blkid", device],
                        false,
                        "verify filesystem signature identity after mkfs",
                    ),
                    command(
                        ["disk-nix", "inspect", device, "--json"],
                        false,
                        "verify formatted filesystem graph relationships",
                    ),
                ],
                vec![
                    "formatted device reports the intended filesystem type".to_string(),
                    "mount, UUID, label, and dependent NixOS references are reviewed after formatting"
                        .to_string(),
                ],
            )
        }
        Operation::Destroy | Operation::Close if collection == Some("luks.devices") => (
            vec![
                command(
                    ["cryptsetup", "status", target],
                    false,
                    "confirm LUKS mapper is closed or absent after close",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "verify dependent graph no longer references the mapper",
                ),
            ],
            vec![
                "mapper is inactive or absent after close".to_string(),
                "backing LUKS device remains intact unless a separate format action was requested"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("btrfsSubvolumes") => (
            vec![command(
                ["disk-nix", "topology", "--json"],
                false,
                "re-probe full topology after Btrfs subvolume deletion",
            )],
            vec![
                "deleted Btrfs subvolume path no longer appears in subvolume listings".to_string(),
                "snapshots, qgroups, and mount consumers are reviewed after deletion".to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            let source = action.context.target.as_deref().unwrap_or(target);
            (
                vec![
                    command(
                        ["disk-nix", "inspect", source, "--json"],
                        false,
                        "verify source target after snapshot deletion",
                    ),
                    command(
                        ["disk-nix", "topology", "--json"],
                        false,
                        "re-probe full topology after snapshot deletion",
                    ),
                ],
                vec![
                    format!("snapshot {snapshot} no longer appears in topology"),
                    "source target remains present with expected consumers and mount state"
                        .to_string(),
                ],
            )
        }
        Operation::Remount if action.context.collection.as_deref() == Some("filesystems") => {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    command(
                        ["disk-nix", "inspect", mountpoint.unwrap_or(target), "--json"],
                        false,
                        "verify filesystem graph state after remount",
                    ),
                ],
                vec![
                    "findmnt reports the mountpoint with the reviewed filesystem options"
                        .to_string(),
                    "local services continue to see the expected filesystem source and type"
                        .to_string(),
                ],
            )
        }
        Operation::Mount if action.context.collection.as_deref() == Some("filesystems") => {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    command(
                        ["disk-nix", "inspect", mountpoint.unwrap_or(target), "--json"],
                        false,
                        "verify filesystem graph state after mount",
                    ),
                ],
                vec![
                    "findmnt reports the mountpoint with the reviewed source and options"
                        .to_string(),
                    "local services see the expected mounted filesystem source and type"
                        .to_string(),
                ],
            )
        }
        Operation::Unmount if action.context.collection.as_deref() == Some("filesystems") => {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    command(
                        ["disk-nix", "topology", "--json"],
                        false,
                        "re-probe full topology after filesystem unmount",
                    ),
                ],
                vec![
                    "findmnt no longer reports the reviewed filesystem as mounted".to_string(),
                    "dependent services and bind mounts have no stale references".to_string(),
                ],
            )
        }
        Operation::Rescan if action.context.collection.as_deref() == Some("filesystems") => {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    filesystem_inspect_command(
                        mountpoint,
                        true,
                        "verify filesystem graph state after rescan",
                    ),
                ],
                vec![
                    "findmnt and disk-nix graph state were refreshed without mounting, remounting, or unmounting"
                        .to_string(),
                    "source, filesystem type, options, and consumers match the reviewed inventory"
                        .to_string(),
                ],
            )
        }
        _ => return None,
    })
}
