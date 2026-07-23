fn remove_rollback_action_commands(
    action: &PlannedAction,
    collection: Option<&str>,
    target: Option<&str>,
    cache_target: Option<&str>,
) -> Option<ActionCommandResult> {
    let _ = cache_target;
    Some(match action.operation {
        Operation::Rollback if collection == Some("lvmSnapshots") => {
            let target = target.unwrap_or("<lvm-snapshot>");
            (
                vec![
                    command(
                        ["lvs", "--reportformat", "json", target],
                        false,
                        "inspect LVM snapshot before merge rollback",
                    ),
                    command(
                        ["lvconvert", "--merge", target],
                        true,
                        "merge the LVM snapshot back into its origin",
                    ),
                ],
                vec![
                    "take a fresh snapshot of the origin before merging".to_string(),
                    "schedule downtime when the origin must be deactivated for merge".to_string(),
                ],
                true,
            )
        }
        Operation::Rollback if collection == Some("snapshots") => {
            let target = target.unwrap_or("<snapshot>");
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            if is_zfs_snapshot_name(snapshot) {
                (
                    vec![
                        command(
                            ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                            false,
                            "inspect ZFS snapshot before rollback",
                        ),
                        zfs_snapshot_rollback_command(
                            snapshot,
                            action.context.recursive_rollback.unwrap_or(false),
                        ),
                    ],
                    vec![
                        "take a fresh snapshot of the current dataset before rollback".to_string(),
                        if action.context.recursive_rollback == Some(true) {
                            "recursive rollback destroys newer snapshots in the dataset lineage; review clones and dependent retention first"
                                .to_string()
                        } else {
                            "review newer snapshots and clones before considering zfs rollback -r or -R"
                                .to_string()
                        },
                    ],
                    true,
                )
            } else {
                (
                    vec![command_with_readiness(
                        ["<snapshot-rollback-tool>", snapshot],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["ZFS snapshot name"],
                        "roll back the snapshot after selecting a concrete ZFS snapshot name",
                    )],
                    vec![
                        "snapshot rollback command is only rendered for unambiguous ZFS snapshot names"
                            .to_string(),
                    ],
                    true,
                )
            }
        }
        Operation::Clone if collection == Some("snapshots") => {
            let target = target.unwrap_or("<clone-dataset>");
            let snapshot = action.context.name.as_deref().unwrap_or("<snapshot>");
            if is_zfs_snapshot_name(snapshot) {
                (
                    vec![
                        command(
                            ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                            false,
                            "inspect ZFS snapshot before clone",
                        ),
                        command(
                            ["zfs", "clone", snapshot, target],
                            true,
                            "clone the reviewed ZFS snapshot to a writable dataset",
                        ),
                    ],
                    vec![
                        "use the clone for inspection, migration, or rollback rehearsal"
                            .to_string(),
                        "destroy temporary clones after validation to release snapshot dependencies"
                            .to_string(),
                    ],
                    true,
                )
            } else if is_btrfs_snapshot_pair(snapshot, target) {
                (
                    vec![
                        command(
                            ["btrfs", "subvolume", "show", snapshot],
                            false,
                            "inspect source Btrfs snapshot subvolume before clone",
                        ),
                        snapshot_command(
                            Some("snapshots"),
                            snapshot,
                            target,
                            action.context.read_only.unwrap_or(false),
                        ),
                    ],
                    vec![
                        "use the cloned subvolume for inspection, migration, or rollback rehearsal"
                            .to_string(),
                        "delete temporary Btrfs clone subvolumes after validation when they are no longer needed"
                            .to_string(),
                    ],
                    true,
                )
            } else {
                (
                    vec![command_with_readiness(
                        ["<snapshot-clone-tool>", snapshot, target],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["ZFS snapshot name or Btrfs snapshot path"],
                        "clone the snapshot after selecting a concrete ZFS snapshot name or Btrfs snapshot path",
                    )],
                    vec![
                        "snapshot clone command is rendered for unambiguous ZFS snapshot names or absolute Btrfs snapshot paths"
                            .to_string(),
                    ],
                    true,
                )
            }
        }
        Operation::RemoveDevice if collection == Some("pools") => {
            let target = zfs_pool_command_target(action, target);
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "remove-device"));
            (
                vec![
                    command(
                        ["zpool", "status", "-P", target],
                        false,
                        "inspect ZFS pool layout and health before device removal",
                    ),
                    zpool_remove_device_command(target, device),
                ],
                vec![
                    "verify the pool supports device removal for the selected vdev class"
                        .to_string(),
                    "monitor evacuation and keep replacement capacity available until verification passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::RemoveDevice if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "remove-device"));
            (
                vec![
                    lvm_physical_volume_inspect_command(device),
                    lvm_physical_volume_move_command(device),
                    lvm_volume_group_reduce_command(target, device),
                ],
                vec![
                    "run pvmove or add replacement capacity before reducing a PV with allocated extents"
                        .to_string(),
                    "verify logical volumes and thin pools still have the intended redundancy and free space"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::RemoveDevice if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "remove-device"));
            (
                vec![
                    md_raid_detail_command(
                        target,
                        "inspect MD RAID redundancy before member removal",
                    ),
                    md_raid_fail_member_command(target, device),
                    md_raid_remove_member_command(target, device),
                ],
                vec![
                    "remove a member only when redundancy and free capacity remain sufficient"
                        .to_string(),
                    "monitor /proc/mdstat until recovery or reshape completes".to_string(),
                ],
                true,
            )
        }
        Operation::RemoveDevice if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            let path = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "remove-device"));
            (
                vec![
                    multipath_list_command(target, "inspect live multipath paths before deletion"),
                    multipath_delete_path_command(path),
                ],
                vec![
                    "remove a path only when alternate paths remain active".to_string(),
                    "verify the path belongs to the intended map WWID before deletion".to_string(),
                ],
                true,
            )
        }
        Operation::RemoveDevice if collection == Some("lvmCaches") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent"),
                        "inspect LVM cache dirty state before detach",
                    ),
                    lvm_cache_uncache_command(target),
                ],
                vec![
                    "switch writeback caches to writethrough before detach".to_string(),
                    "wait for dirty data to drain before lvconvert --uncache".to_string(),
                ],
                true,
            )
        }
        Operation::RemoveDevice if collection == Some("filesystems") => {
            let fs_type = action.context.fs_type.as_deref();
            let target = target.unwrap_or(match fs_type {
                Some("bcachefs") => "<bcachefs-mountpoint>",
                _ => "<btrfs-filesystem>",
            });
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "remove-device"));
            if fs_type == Some("bcachefs") {
                (
                    vec![
                        bcachefs_usage_command(
                            target,
                            "inspect bcachefs allocation and free space before device removal",
                        ),
                        bcachefs_rereplicate_command(target),
                        bcachefs_remove_device_command(target, device),
                    ],
                    vec![
                        "remove a bcachefs device only when remaining replicas and capacity are sufficient"
                            .to_string(),
                        "rereplicate or migrate data before removing the reviewed member"
                            .to_string(),
                    ],
                    true,
                )
            } else {
                (
                    vec![
                        command(
                            ["btrfs", "filesystem", "usage", "-b", target],
                            false,
                            "inspect Btrfs allocation and free space before device removal",
                        ),
                        btrfs_remove_device_command(target, device),
                    ],
                    vec![
                        "remove a Btrfs device only when remaining data and metadata space are sufficient"
                            .to_string(),
                        "run or review balance progress until device evacuation completes".to_string(),
                    ],
                    true,
                )
            }
        }
        Operation::RemoveDevice if collection == Some("caches") => {
            let target = cache_target.unwrap_or("<cache-device>");
            (
                vec![
                    bcache_sysfs_read_command(
                        target,
                        "dirty_data",
                        "inspect dirty data before bcache detach",
                    ),
                    bcache_detach_command(target),
                ],
                vec![
                    "switch writeback caches to writethrough and wait for dirty data to drain before detach"
                        .to_string(),
                    "keep backing storage online and verify it remains readable after detach"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("caches") => {
            let target = cache_target.unwrap_or("<cache-device>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect modeled cache layer relationships after status refresh",
                    ),
                    bcache_sysfs_read_command(target, "state", "refresh bcache state"),
                    bcache_sysfs_read_command(target, "cache_mode", "refresh bcache cache mode"),
                    bcache_sysfs_read_command(target, "dirty_data", "refresh bcache dirty data"),
                ],
                vec![
                    "use add-device or remove-device when cache-set attachment must change"
                        .to_string(),
                    "verify dirty data before detach, replacement, or cache-mode changes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::Unexport if collection == Some("exports") => {
            let target = export_target_path(action);
            (
                vec![nfs_export_destroy_command(
                    target,
                    action.context.client.as_deref(),
                )],
                vec![
                    "drain or migrate clients before unexporting the path".to_string(),
                    "verify no active mounts still depend on the export after reload".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::Unmount if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            (
                vec![
                    nfs_mount_findmnt_command(mountpoint),
                    nfs_mount_destroy_command(mountpoint),
                ],
                vec![
                    "stop services and automount units that depend on the NFS mount before unmounting"
                        .to_string(),
                    "verify no open files, bind mounts, or user sessions still reference the mountpoint"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::Detach
            if collection == Some("luns") || action.id.starts_with("luns:") =>
        {
            let target = target.unwrap_or("<lun>");
            let devices = lun_rescan_devices(action);
            let mut commands = vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "inspect LUN consumers before detaching reviewed SCSI paths",
            )];
            commands.push(lsscsi_lun_inventory_command(
                "inspect host-visible LUN transport and size before detaching paths",
            ));
            if devices.is_empty() {
                commands.push(command_with_readiness(
                    ["<scsi-delete-device>", "<lun-path>"],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "detach a LUN path after declaring a stable by-path device",
                ));
            } else {
                for device in devices {
                    commands.push(scsi_device_delete_command(&device));
                }
            }
            commands.push(command(
                ["multipath", "-r"],
                true,
                "reload multipath maps after LUN path detach",
            ));
            commands.push(command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify detached LUN paths and remaining consumers",
            ));
            (
                commands,
                vec![
                    "unmount filesystems and deactivate dm, LVM, or multipath consumers before detach"
                        .to_string(),
                    "detach only reviewed stable paths; target-side LUN deletion remains an external storage-array action"
                        .to_string(),
                ],
                true,
            )
        }
        _ => return None,
    })
}
