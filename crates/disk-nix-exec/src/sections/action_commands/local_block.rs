fn local_block_action_commands(
    action: &PlannedAction,
    collection: Option<&str>,
    target: Option<&str>,
    cache_target: Option<&str>,
) -> Option<ActionCommandResult> {
    let _ = cache_target;
    Some(match action.operation {
        Operation::Grow if collection == Some("volumes") || action.id.starts_with("volumes:") => {
            let target = lvm_volume_target_path(target);
            let desired_size = action.context.desired_size.as_deref();
            let note = desired_size
                .map(|size| format!("desired size from spec: {size}"))
                .unwrap_or_else(|| {
                    "replace <size> after comparing desired state with probed capacity".to_string()
                });
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        None,
                        "inspect current LVM logical volume state",
                    ),
                    lvm_logical_volume_extend_command(target, desired_size),
                ],
                vec![note],
                true,
            )
        }
        Operation::Rescan if collection == Some("volumes") || action.id.starts_with("volumes:") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        None,
                        "refresh LVM logical volume attributes and activation state",
                    ),
                    command(
                        ["disk-nix", "inspect", target.unwrap_or("<logical-volume>")],
                        false,
                        "inspect modeled LV graph relationships after status refresh",
                    ),
                ],
                vec![
                    "use grow when LV capacity must change".to_string(),
                    "use activate or deactivate when LV availability must change".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let device = action.context.device.as_deref();
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect current volume group size and free extents before growth",
                    ),
                    volume_group_extend_command(target, device),
                ],
                vec![
                    "initialize or verify the physical volume before extending the VG".to_string(),
                    "grow dependent logical volumes only after VG free extents reflect added capacity"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("physicalVolumes") => {
            let target = lvm_physical_volume_target(action);
            (
                vec![
                    lvm_physical_volume_inspect_command(target),
                    lvm_physical_volume_resize_command(target),
                ],
                vec![
                    "grow the backing partition, LUN, or disk before pvresize".to_string(),
                    "verify volume group free extents before extending logical volumes".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("physicalVolumes") => {
            let target = lvm_physical_volume_target(action);
            let inspect = match target {
                Some(target) => command(
                    ["pvs", "--reportformat", "json", target],
                    false,
                    "inspect physical volume metadata before cache refresh",
                ),
                None => command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "inspect current LVM physical volume inventory before cache refresh",
                ),
            };
            let mut commands = vec![inspect, lvm_physical_volume_rescan_command(target)];
            commands.push(command(
                ["pvs", "--reportformat", "json"],
                false,
                "inspect refreshed LVM physical volume inventory",
            ));
            (
                commands,
                vec![
                    "rescan backing block paths first when device visibility changed".to_string(),
                    "use grow when pvresize is needed after capacity changes".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before metadata refresh",
                    ),
                    command(
                        ["pvscan", "--cache"],
                        true,
                        "refresh the LVM physical volume device cache",
                    ),
                    command(
                        ["vgscan"],
                        true,
                        "scan available LVM volume groups without creating metadata",
                    ),
                    command(
                        ["vgchange", "--refresh", target],
                        true,
                        "reactivate the reviewed volume group with refreshed metadata",
                    ),
                ],
                vec![
                    "run host path rescans before VG refresh when devices were added or resized"
                        .to_string(),
                    "verify LV activation state and VG free extents after refresh".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("thinPools") => {
            let target = lvm_volume_target_path(target);
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_size,data_percent,metadata_percent,seg_monitor"),
                        "inspect current thin pool data and metadata utilization",
                    ),
                    thin_pool_extend_command(target, desired_size),
                ],
                vec![
                    "extend metadata before it approaches exhaustion".to_string(),
                    "verify thin pool autoextend policy and monitoring before growth".to_string(),
                    "review thin volume overcommit before adding virtual capacity".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("thinPools") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_size,data_percent,metadata_percent,seg_monitor"),
                        "refresh thin pool data, metadata, and monitoring state",
                    ),
                    command(
                        ["disk-nix", "inspect", target.unwrap_or("<thin-pool>")],
                        false,
                        "inspect modeled thin pool relationships after status refresh",
                    ),
                ],
                vec![
                    "use grow when data or metadata capacity must change".to_string(),
                    "review utilization before allocating more thin volumes".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("thinPools") => {
            let target = target.unwrap_or("<thin-pool>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json"],
                        false,
                        "inspect volume group free space before creating the thin pool",
                    ),
                    thin_pool_create_command(target, desired_size),
                ],
                vec![
                    "verify the target volume group has enough data and metadata capacity"
                        .to_string(),
                    "choose overcommit, monitoring, and autoextend policy before using the thin pool"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("lvmCaches") => {
            let target = lvm_volume_target_path(target);
            let cache_pool = action
                .context
                .device
                .as_deref()
                .or_else(|| action.context.devices.first().map(String::as_str));
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some(
                            "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent",
                        ),
                        "inspect origin LV and cache state before attaching LVM cache",
                    ),
                    lvm_cache_attach_command(target, cache_pool),
                ],
                vec![
                    "verify the cache pool LV is clean and belongs to the same VG as the origin"
                        .to_string(),
                    "prefer writethrough cache mode until post-attach verification passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("lvmCaches") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some(
                            "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent",
                        ),
                        "refresh LVM cache mode, policy, utilization, and metadata state",
                    ),
                    command(
                        ["disk-nix", "inspect", target.unwrap_or("<lvm-cache>")],
                        false,
                        "inspect modeled LVM cache relationships after status refresh",
                    ),
                ],
                vec![
                    "use property updates when cache mode or policy must change".to_string(),
                    "verify dirty data before detach, uncache, or cache-pool replacement"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("physicalVolumes") => {
            let target = lvm_physical_volume_target(action);
            (
                vec![
                    disk_nix_inspect_command(
                        target,
                        "<physical-volume>",
                        "physical volume device",
                        "inspect target device before creating LVM PV metadata",
                    ),
                    lvm_physical_volume_create_command(target),
                ],
                vec![
                    "verify the device contains no data that must be preserved before pvcreate"
                        .to_string(),
                    "extend or create a volume group only after pvs reports the PV".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("loopDevices") => {
            let target = loop_device_target_path(action);
            (
                vec![
                    loop_device_list_command(
                        target,
                        "inspect loop device before refreshing backing size",
                    ),
                    loop_device_refresh_command(target),
                ],
                vec![
                    "grow the backing file or block device before refreshing the loop mapping"
                        .to_string(),
                    "resize dependent filesystems only after losetup reports the new size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("loopDevices") => {
            let target = loop_device_target_path(action);
            (
                vec![
                    loop_device_list_command(target, "refresh loop device mapping inventory"),
                    loop_device_inspect_command(target),
                ],
                vec![
                    "loop rescan does not refresh size; use grow after backing size changes"
                        .to_string(),
                    "review dependent filesystems and mappings before detach".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("backingFiles") => {
            let target = backing_file_target_path(action);
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    backing_file_absent_command(target),
                    backing_file_create_command(target, desired_size),
                    backing_file_stat_command(target, "inspect backing file after creation"),
                ],
                vec![
                    "create only a new file; existing backing images are left untouched"
                        .to_string(),
                    "verify sparse allocation policy and host filesystem free space before attaching consumers"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("backingFiles") => {
            let target = backing_file_target_path(action);
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    backing_file_stat_command(target, "inspect backing file before growth"),
                    backing_file_grow_command(target, desired_size),
                ],
                vec![
                    "verify host filesystem free space and sparse allocation policy before growth"
                        .to_string(),
                    "refresh loop devices, swap signatures, and dependent filesystems after the file grows"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("backingFiles") => {
            let target = backing_file_target_path(action);
            (
                vec![
                    backing_file_stat_command(target, "refresh backing file size and metadata"),
                    backing_file_usage_command(target),
                    backing_file_inspect_command(target),
                ],
                vec![
                    "backing file rescan is read-only and does not resize or detach consumers"
                        .to_string(),
                    "use grow only when file-backed storage capacity must change".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("dmMaps") => {
            let target = dm_map_target_path(action);
            (
                vec![
                    dmsetup_info_command(target, "refresh device-mapper identity metadata"),
                    dmsetup_deps_command(target),
                    dmsetup_table_command(target),
                    dmsetup_status_command(target),
                    dm_map_inspect_command(target),
                ],
                vec![
                    "device-mapper rescan is read-only and does not reload or remove maps"
                        .to_string(),
                    "use domain-specific LUKS, LVM, VDO, multipath, or cache actions for mutating mapper lifecycle"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("dmMaps") => {
            let target = dm_map_target_path(action);
            let rename_to = dm_map_rename_to(action);
            (
                vec![
                    dmsetup_info_command(target, "inspect device-mapper identity before rename"),
                    dmsetup_deps_command(target),
                    dmsetup_rename_command(target, rename_to.as_deref()),
                    dm_map_inspect_command(target),
                ],
                vec![
                    "device-mapper rename changes the visible mapper path and can break consumers until declarations are updated"
                        .to_string(),
                    "prefer LUKS, LVM, VDO, multipath, or cache-specific rename workflows when the mapper is owned by a higher-level domain"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("dmMaps") => {
            let target = dm_map_target_path(action);
            (
                vec![
                    dmsetup_info_command(target, "inspect device-mapper identity before removal"),
                    dmsetup_deps_command(target),
                    dmsetup_status_command(target),
                    dmsetup_remove_command(target),
                ],
                vec![
                    "device-mapper removal destroys the live map and can make dependent data inaccessible"
                        .to_string(),
                    "prefer domain-specific LUKS, LVM, VDO, multipath, or cache teardown when the mapper is owned elsewhere"
                        .to_string(),
                ],
                true,
            )
        }
        _ => return None,
    })
}
