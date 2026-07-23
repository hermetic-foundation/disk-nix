fn device_change_action_commands(
    action: &PlannedAction,
    collection: Option<&str>,
    target: Option<&str>,
    cache_target: Option<&str>,
) -> Option<ActionCommandResult> {
    let _ = cache_target;
    Some(match action.operation {
        Operation::AddDevice if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "add-device"));
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect current volume group state before adding a physical volume",
                    ),
                    volume_group_extend_command(target, device),
                ],
                vec![
                    "initialize or verify the physical volume before extending the VG".to_string(),
                ],
                true,
            )
        }
        Operation::AddDevice if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "add-device"));
            (
                vec![
                    md_raid_detail_command(
                        target,
                        "inspect MD RAID redundancy before adding a member",
                    ),
                    md_raid_add_member_command(target, device),
                ],
                vec![
                    "add a member or spare only after confirming array health and intended role"
                        .to_string(),
                    "monitor /proc/mdstat until recovery or reshape completes".to_string(),
                ],
                true,
            )
        }
        Operation::AddDevice if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            let path = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "add-device"));
            (
                vec![
                    multipath_list_command(
                        target,
                        "inspect live multipath paths before adding a path",
                    ),
                    multipath_add_path_command(path),
                ],
                vec![
                    "verify the path belongs to the intended LUN before adding it to multipathd"
                        .to_string(),
                    "reload or resize maps only after every expected path is visible".to_string(),
                ],
                true,
            )
        }
        Operation::AddDevice => {
            let target = if collection == Some("caches") {
                cache_target.unwrap_or("<cache-device>")
            } else if collection == Some("pools") {
                zfs_pool_command_target(action, target)
            } else {
                target.unwrap_or("<target>")
            };
            let fs_type = action.context.fs_type.as_deref();
            let device = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "add-device"));
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect target health before adding a device",
                    ),
                    add_device_command(collection, fs_type, target, device),
                ],
                vec![
                    "verify the new device identity and redundancy policy before attaching it"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::ReplaceDevice
            if collection == Some("filesystems")
                && action.context.fs_type.as_deref() == Some("bcachefs") =>
        {
            let target = target.unwrap_or("<bcachefs-mountpoint>");
            let from = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "replace-device"));
            let to = action.context.replacement.as_deref();
            (
                vec![
                    bcachefs_usage_command(
                        target,
                        "inspect bcachefs allocation before replacement",
                    ),
                    bcachefs_add_device_command(target, to),
                    bcachefs_rereplicate_command(target),
                    bcachefs_remove_device_command(target, from),
                ],
                vec![
                    "add replacement capacity before evacuating the old bcachefs member"
                        .to_string(),
                    "wait for rereplication to converge before removing the old device".to_string(),
                    "keep the old device available until post-apply verification passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::ReplaceDevice if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let from = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "replace-device"));
            let to = action.context.replacement.as_deref();
            (
                vec![
                    md_raid_detail_command(
                        target,
                        "inspect MD RAID redundancy before member replacement",
                    ),
                    md_raid_replace_member_command(target, from, to),
                ],
                vec![
                    "replace one member at a time while the array is healthy".to_string(),
                    "monitor /proc/mdstat until replacement sync completes".to_string(),
                ],
                true,
            )
        }
        Operation::ReplaceDevice if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            let from = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "replace-device"));
            let to = action.context.replacement.as_deref();
            (
                vec![
                    multipath_list_command(
                        target,
                        "inspect live multipath paths before replacement",
                    ),
                    multipath_add_path_command(to),
                    multipath_delete_path_command(from),
                ],
                vec![
                    "add and verify the replacement path before deleting the old path".to_string(),
                    "keep alternate paths active while replacing a single path".to_string(),
                ],
                true,
            )
        }
        Operation::ReplaceDevice if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let from = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "replace-device"));
            let to = action.context.replacement.as_deref();
            (
                vec![
                    lvm_physical_volume_inspect_command(from),
                    lvm_physical_volume_inspect_command(to),
                    lvm_volume_group_extend_replacement_command(target, to),
                    lvm_physical_volume_move_to_command(from, to),
                    lvm_volume_group_reduce_command(target, from),
                ],
                vec![
                    "add the replacement physical volume before moving extents".to_string(),
                    "keep the old PV online until pvmove completes and no allocated extents remain"
                        .to_string(),
                    "verify logical volumes, thin pools, and filesystems before vgreduce"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::ReplaceDevice => {
            let target = if collection == Some("caches") {
                cache_target.unwrap_or("<cache-device>")
            } else {
                target.unwrap_or("<target>")
            };
            let fs_type = action.context.fs_type.as_deref();
            let from = action
                .context
                .device
                .as_deref()
                .or_else(|| action_id_suffix(&action.id, "replace-device"));
            let to = action.context.replacement.as_deref();
            let replacement_cache_set = action.context.cache_set_uuid.as_deref();
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect redundancy and source device health before replacement",
                    ),
                    if collection == Some("caches") {
                        match (from, to) {
                            (Some(from), Some(to)) => {
                                bcache_replace_command(target, from, to, replacement_cache_set)
                            }
                            _ => replace_device_command(collection, fs_type, target, from, to),
                        }
                    } else {
                        replace_device_command(collection, fs_type, target, from, to)
                    },
                ],
                vec![
                    "keep the old device available until post-apply verification passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rebalance => {
            let target = if collection == Some("pools") {
                zfs_pool_command_target(action, target)
            } else {
                target.unwrap_or("<target>")
            };
            let rebalance = rebalance_command(
                collection,
                action.context.fs_type.as_deref(),
                target,
                &action.context.property_assignments,
            );
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect pool or filesystem health before rebalance",
                    ),
                    rebalance,
                ],
                vec![
                    "monitor progress and health until the rebalance operation completes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Scrub => {
            let target = target.unwrap_or("<target>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect pool or filesystem health before scrub",
                    ),
                    scrub_command(collection, action.context.fs_type.as_deref(), target),
                ],
                vec!["monitor scrub progress and health until completion".to_string()],
                true,
            )
        }
        Operation::Trim => {
            let target = target.unwrap_or("<filesystem>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect filesystem and backing discard support before trim",
                    ),
                    filesystem_trim_command(collection, target),
                ],
                vec![
                    "verify discard is safe through LUKS, LVM, thin, VDO, and virtual layers"
                        .to_string(),
                    "prefer scheduled fstrim for routine maintenance".to_string(),
                ],
                true,
            )
        }
        Operation::SetProperty => {
            let target = if collection == Some("caches") {
                cache_target.unwrap_or("<cache-device>")
            } else {
                target.unwrap_or("<target>")
            };
            let Some(property) = action.context.property.as_deref() else {
                return Some((
                    vec![command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect declared storage object state",
                    )],
                    vec!["no property mutation was requested by this declaration".to_string()],
                    false,
                ));
            };
            let property_assignment = property_assignment(action);
            let property_command = if collection == Some("exports") {
                nfs_export_property_command(
                    target,
                    action.context.client.as_deref(),
                    property,
                    action.context.property_value.as_deref(),
                    action.context.options.as_deref(),
                )
            } else if collection == Some("btrfsQgroups") {
                btrfs_qgroup_property_command(
                    target,
                    action.context.name.as_deref().unwrap_or("<qgroupid>"),
                    property,
                    &property_assignment,
                )
            } else if collection == Some("snapshots") {
                snapshot_property_command(
                    action.context.name.as_deref().unwrap_or(target),
                    property,
                    action.context.property_value.as_deref(),
                )
            } else if collection == Some("filesystems") {
                filesystem_property_command(
                    action.context.fs_type.as_deref(),
                    target,
                    action.context.device.as_deref(),
                    property,
                    &property_assignment,
                )
            } else if collection == Some("swaps") {
                swap_property_command(
                    swap_target_path(action),
                    property,
                    action.context.property_value.as_deref(),
                )
            } else if collection == Some("luks.devices") {
                luks_device_property_command(
                    action.context.device.as_deref(),
                    property,
                    action.context.property_value.as_deref(),
                )
            } else if collection == Some("luksKeyslots") {
                luks_keyslot_property_command(action, property)
            } else if collection == Some("luksTokens") {
                luks_token_import_command(
                    luks_token_device(action),
                    luks_token_id(action),
                    action
                        .context
                        .property_value
                        .as_deref()
                        .or(action.context.token_file.as_deref()),
                )
            } else {
                let property_target = if collection == Some("pools") {
                    action.context.name.as_deref().unwrap_or(target)
                } else {
                    target
                };
                set_property_command(
                    collection,
                    property_target,
                    property,
                    &property_assignment,
                    action.context.cache_set_uuid.as_deref(),
                )
            };
            let inspect_target = if collection == Some("snapshots") {
                action.context.name.as_deref().unwrap_or(target)
            } else {
                target
            };
            (
                vec![
                    command(
                        ["disk-nix", "inspect", inspect_target],
                        false,
                        "inspect current properties before applying changes",
                    ),
                    property_command,
                ],
                vec![
                    "property values must come from the desired spec and target domain".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("snapshots") => {
            let snapshot = snapshot_rescan_identity(action, "<snapshot>");
            let mut commands = vec![command(
                ["disk-nix", "inspect", snapshot],
                false,
                "inspect modeled snapshot graph relationships after metadata refresh",
            )];
            if is_zfs_snapshot_name(snapshot) {
                commands.push(zfs_snapshot_list_command(
                    snapshot,
                    "refresh ZFS snapshot size and reference metadata",
                ));
                commands.push(command(
                    [
                        "zfs",
                        "get",
                        "-H",
                        "-p",
                        "-o",
                        "property,value",
                        "creation,used,referenced,userrefs,defer_destroy",
                        snapshot,
                    ],
                    false,
                    "refresh ZFS snapshot properties and retention metadata",
                ));
                commands.push(snapshot_hold_list_command(snapshot));
            } else if snapshot.starts_with('/') {
                commands.push(command(
                    ["btrfs", "subvolume", "show", snapshot],
                    false,
                    "refresh Btrfs snapshot subvolume metadata",
                ));
                commands.push(command(
                    ["btrfs", "property", "get", "-ts", snapshot, "ro"],
                    false,
                    "refresh Btrfs snapshot read-only property",
                ));
            } else {
                commands.push(command_with_readiness(
                    ["<snapshot-rescan-tool>", snapshot],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["ZFS snapshot name or Btrfs snapshot path"],
                    "refresh snapshot metadata after selecting the target-specific tool",
                ));
            }
            (
                commands,
                vec![
                    "use hold or release operations for retention changes".to_string(),
                    "use clone or rollback only after reviewing refreshed snapshot metadata"
                        .to_string(),
                ],
                true,
            )
        }
        _ => return None,
    })
}
