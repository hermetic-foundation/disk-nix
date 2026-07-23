fn commands_for_action(action: &PlannedAction) -> (Vec<ExecutionCommand>, Vec<String>, bool) {
    let parts: Vec<&str> = action.id.split(':').collect();
    let collection = action
        .context
        .collection
        .as_deref()
        .or_else(|| parts.first().copied());
    let target = action
        .context
        .target
        .as_deref()
        .or(action.context.name.as_deref())
        .or_else(|| parts.get(1).copied());
    let cache_target = bcache_target_path(action);
    match action.operation {
        Operation::Grow
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            let desired_size = action.context.desired_size.as_deref();
            let grow_command = filesystem_grow_command(fs_type, target, device, desired_size);
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "re-read graph state for the filesystem before resizing",
                    ),
                    grow_command,
                ],
                vec![
                    format!(
                        "select the {fs_type} grow command: xfs_growfs, resize2fs, btrfs filesystem resize, zfs set volsize, or equivalent"
                    ),
                    "verify available backing capacity before running the grow command".to_string(),
                ],
                true,
            )
        }
        Operation::Format
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            let mut commands = vec![
                disk_nix_inspect_command(
                    device,
                    "<filesystem-device>",
                    "filesystem source device",
                    "inspect target device before creating a filesystem signature",
                ),
            ];
            if device.is_some_and(|device| device.starts_with("/dev/md/")) {
                commands.push(command(
                    ["udevadm", "settle"],
                    false,
                    "wait for md device events to settle before formatting",
                ));
            }
            commands.push(filesystem_format_command(fs_type, device));
            if matches!(fs_type, "btrfs" | "bcachefs") {
                if let Some(mountpoint) = filesystem_mountpoint(action) {
                    commands.push(filesystem_mount_command(
                        device,
                        Some(mountpoint),
                        Some(fs_type),
                        action.context.options.as_deref(),
                    ));
                }
            }
            (
                commands,
                vec![
                    format!("formatting {target} as {fs_type} destroys existing data on the selected device"),
                    "prefer preserving or migrating data before replacing a filesystem signature"
                        .to_string(),
                    "mount the new filesystem only after its UUID, label, and stable device path are verified"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Shrink
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            let desired_size = action.context.desired_size.as_deref();
            (
                filesystem_shrink_commands(fs_type, target, device, desired_size),
                vec![
                    "shrink only after backups or snapshots are verified".to_string(),
                    "prefer migrate-to-smaller-filesystem workflows when online shrink support is absent"
                        .to_string(),
                    "restore dependent mounts and services only after post-shrink checks pass"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Check
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            (
                filesystem_check_commands(fs_type, target, device),
                vec![
                    "run read-only consistency checks before any repair workflow".to_string(),
                    "quiesce or unmount the filesystem when the checker requires offline access"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Repair
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let target = target.unwrap_or("<filesystem>");
            let fs_type = action
                .context
                .fs_type
                .as_deref()
                .unwrap_or("<filesystem-type>");
            let device = action.context.device.as_deref();
            (
                filesystem_repair_commands(fs_type, target, device),
                vec![
                    "repair only after a read-only check and backup review".to_string(),
                    "prefer repairing a cloned device before the production filesystem when practical"
                        .to_string(),
                    "restore mounts and services only after post-repair verification passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Remount
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    filesystem_remount_command(mountpoint, action.context.options.as_deref()),
                ],
                vec![
                    "review active services before changing filesystem mount options".to_string(),
                    "persist the final options through the NixOS fileSystems entry".to_string(),
                ],
                true,
            )
        }
        Operation::Mount
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![filesystem_mount_command(
                    action.context.device.as_deref(),
                    mountpoint,
                    action.context.fs_type.as_deref(),
                    action.context.options.as_deref(),
                )],
                vec![
                    "verify the source device, filesystem type, and mountpoint before mounting"
                        .to_string(),
                    "persist long-lived mounts through the NixOS fileSystems entry".to_string(),
                ],
                true,
            )
        }
        Operation::Unmount
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    filesystem_unmount_command(mountpoint),
                ],
                vec![
                    "stop services, automount units, and sessions that depend on the mountpoint before unmounting"
                        .to_string(),
                    "verify no open files, bind mounts, or namespaces still reference the mountpoint"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan
            if collection == Some("filesystems") || action.id.starts_with("filesystems:") =>
        {
            let mountpoint = filesystem_mountpoint(action);
            (
                vec![
                    filesystem_findmnt_command(mountpoint),
                    filesystem_inspect_command(
                        mountpoint,
                        false,
                        "refresh modeled filesystem graph state",
                    ),
                ],
                vec![
                    "filesystem rescan is read-only and does not mount, remount, unmount, or format storage"
                        .to_string(),
                    "use the refreshed inventory before selecting any mutating lifecycle action"
                        .to_string(),
                ],
                true,
            )
        }
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
        Operation::Rescan if collection == Some("luns") || action.id.starts_with("luns:") => {
            let target = target.unwrap_or("<lun>");
            let mut commands = vec![
                command(
                    ["iscsiadm", "--mode", "session", "--rescan"],
                    true,
                    "rescan iSCSI sessions to refresh existing LUN paths",
                ),
                lsscsi_lun_inventory_command(
                    "inspect host-visible LUN transport and size before per-device rescans",
                ),
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "inspect current LUN paths before per-device rescans",
                ),
            ];
            let devices = lun_rescan_devices(action);
            if devices.is_empty() {
                commands.push(command_with_readiness(
                    ["<scsi-rescan-device>", "<lun-path>"],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "rescan the concrete SCSI path after declaring a stable by-path LUN device",
                ));
            }
            for device in devices {
                commands.push(scsi_device_rescan_command(&device));
            }
            commands.extend([
                command(
                    ["multipath", "-r"],
                    true,
                    "reload multipath maps after refreshed LUN paths",
                ),
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "verify that refreshed paths and consumers are visible",
                ),
            ]);
            (
                commands,
                vec![
                    "declare stable LUN path devices to render per-path SCSI rescans".to_string(),
                    "verify multipath maps before exposing dependent consumers".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("luns") || action.id.starts_with("luns:") => {
            let target = target.unwrap_or("<lun>");
            let mut commands = vec![
                command(
                    ["iscsiadm", "--mode", "session", "--rescan"],
                    true,
                    "rescan iSCSI sessions after target-side LUN growth",
                ),
                lsscsi_lun_inventory_command(
                    "inspect host-visible LUN transport and size before growth rescans",
                ),
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "inspect current LUN paths before per-device rescans",
                ),
            ];
            let devices = lun_rescan_devices(action);
            if devices.is_empty() {
                commands.push(command_with_readiness(
                    ["<scsi-rescan-device>", "<lun-path>"],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "rescan the concrete SCSI path after declaring a stable by-path LUN device",
                ));
            }
            for device in devices {
                commands.push(scsi_device_rescan_command(&device));
            }
            commands.extend([
                command(
                    ["multipath", "-r"],
                    true,
                    "reload multipath maps when the LUN is multipathed",
                ),
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "verify that consumers see the new capacity",
                ),
            ]);
            (
                commands,
                vec![
                    "coordinate the target-side LUN grow before host rescans".to_string(),
                    "declare stable LUN path devices to render per-path SCSI rescans".to_string(),
                ],
                true,
            )
        }
        Operation::Create | Operation::Attach
            if collection == Some("luns") || action.id.starts_with("luns:") =>
        {
            let target = target.unwrap_or("<lun>");
            let mut commands = vec![
                command(
                    ["iscsiadm", "--mode", "session", "--rescan"],
                    true,
                    "rescan iSCSI sessions after target-side LUN creation",
                ),
                lsscsi_lun_inventory_command(
                    "inspect host-visible LUN transport and size after session rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "inspect the newly attached LUN and consumers",
                ),
            ];
            let devices = lun_rescan_devices(action);
            if devices.is_empty() {
                commands.push(command_vec_with_readiness(
                    vec!["<scsi-rescan-device>", "<lun-path>"],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "rescan the concrete SCSI path after declaring a stable by-path device",
                ));
            }
            for device in &devices {
                commands.push(scsi_device_rescan_command(device));
            }
            commands.push(command(
                ["multipath", "-r"],
                true,
                "reload multipath maps after newly attached LUN paths appear",
            ));
            if devices.is_empty() {
                commands.push(command_vec_with_readiness(
                    vec!["blockdev", "--getsize64", "<lun-path>"],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["stable LUN device path"],
                    "verify the reviewed LUN path after declaring a stable by-path device",
                ));
            }
            for device in &devices {
                commands.push(command_vec(
                    vec!["blockdev", "--getsize64", device.as_str()],
                    false,
                    "verify the reviewed LUN path is visible to the kernel",
                ));
            }
            (
                commands,
                vec![
                    "create or map the target-side LUN before host attach".to_string(),
                    "declare stable LUN path devices to verify every expected path".to_string(),
                    "enable filesystems, LVM, or multipath consumers only after verification"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create
        | Operation::Grow
        | Operation::Attach
        | Operation::Detach
        | Operation::Destroy
        | Operation::SetProperty
        | Operation::Rescan
            if collection == Some("targetLuns") || action.id.starts_with("targetLuns:") =>
        {
            let target = target.unwrap_or("<target-lun>");
            (
                target_lun_commands(action, target),
                vec![
                    "target-side LUN work is provider-specific and stays non-ready until an array adapter or reviewed runbook renders concrete commands"
                        .to_string(),
                    "run host-side luns, iscsiSessions, and multipath rescans only after the target reports the intended mapping and capacity"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow | Operation::Rescan
            if collection == Some("iscsiSessions") || action.id.starts_with("iscsiSessions:") =>
        {
            let target = target.unwrap_or("<iscsi-session>");
            (
                vec![
                    command(
                        ["iscsiadm", "--mode", "session", "--rescan"],
                        true,
                        "rescan iSCSI sessions after target-side changes",
                    ),
                    lsscsi_lun_inventory_command(
                        "inspect host-visible LUN transport and size after session rescan",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify updated iSCSI, LUN, and consumer topology",
                    ),
                ],
                vec!["coordinate session rescans with every dependent LUN consumer".to_string()],
                true,
            )
        }
        Operation::Create | Operation::Login
            if collection == Some("iscsiSessions") || action.id.starts_with("iscsiSessions:") =>
        {
            let target = target.unwrap_or("<iscsi-target-iqn>");
            let portal = action.context.portal.as_deref();
            let discovery = match portal {
                Some(portal) => command_vec(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "discovery",
                        "--type",
                        "sendtargets",
                        "--portal",
                        portal,
                    ],
                    true,
                    "discover iSCSI target records from the reviewed portal",
                ),
                None => command_vec_with_readiness(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "discovery",
                        "--type",
                        "sendtargets",
                        "--portal",
                        "<portal>",
                    ],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["iSCSI portal"],
                    "discover iSCSI target records after selecting the target portal",
                ),
            };
            let login = match portal {
                Some(portal) => command_vec(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        target,
                        "--portal",
                        portal,
                        "--login",
                    ],
                    true,
                    "log in to the reviewed iSCSI target through the selected portal",
                ),
                None => command_vec_with_readiness(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        target,
                        "--portal",
                        "<portal>",
                        "--login",
                    ],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["iSCSI portal"],
                    "log in to the iSCSI target after selecting the target portal",
                ),
            };
            (
                vec![discovery, login],
                vec![
                    "verify the target IQN and portal before creating host sessions".to_string(),
                    "rescan and settle multipath paths before exposing dependent volumes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::Logout
            if collection == Some("iscsiSessions") || action.id.starts_with("iscsiSessions:") =>
        {
            let target = target.unwrap_or("<iscsi-target-iqn>");
            let portal = action.context.portal.as_deref();
            let logout = match portal {
                Some(portal) => command_vec(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        target,
                        "--portal",
                        portal,
                        "--logout",
                    ],
                    true,
                    "log out from the reviewed iSCSI target and portal",
                ),
                None => command_vec(
                    vec![
                        "iscsiadm",
                        "--mode",
                        "node",
                        "--targetname",
                        target,
                        "--logout",
                    ],
                    true,
                    "log out from all node records for the reviewed iSCSI target",
                ),
            };
            (
                vec![logout],
                vec![
                    "unmount filesystems and deactivate mappings before logging out".to_string(),
                    "verify multipath, LVM, and filesystem consumers have migrated away"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("swaps") => {
            let target = swap_target_path(action);
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["swapon", "--show", "--bytes", "--raw"],
                        false,
                        "inspect active swap state before resizing",
                    ),
                    swap_command(
                        "swapoff",
                        target,
                        "disable swap before changing backing storage or signature",
                    ),
                    swap_resize_command(target, desired_size),
                    swap_command(
                        "mkswap",
                        target,
                        "recreate the swap signature after backing storage resize",
                    ),
                    swap_command("swapon", target, "reactivate swap after verification"),
                ],
                vec![
                    "verify memory pressure and hibernation dependencies before swapoff"
                        .to_string(),
                    "prefer adding replacement swap capacity before resizing active swap"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Deactivate if collection == Some("swaps") => {
            let target = swap_target_path(action);
            (
                vec![
                    command(
                        ["swapon", "--show", "--bytes", "--raw"],
                        false,
                        "inspect active swap state before swapoff",
                    ),
                    swap_command("swapoff", target, "disable active swap without removing its signature"),
                ],
                vec![
                    "verify memory pressure and hibernation dependencies before swapoff"
                        .to_string(),
                    "use destroy only when swap signature metadata should be removed".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("swaps") => {
            let target = swap_target_path(action);
            (
                vec![
                    disk_nix_inspect_command(
                        target,
                        "<swap>",
                        "swap target path",
                        "inspect target before disabling and wiping swap signature",
                    ),
                    swap_command("swapoff", target, "disable active swap before removing its signature"),
                    swap_wipefs_command(target),
                ],
                vec![
                    "remove or update NixOS swapDevices before wiping the signature".to_string(),
                    "verify resume and hibernation references before deleting swap metadata"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("swaps") => {
            let target = swap_target_path(action);
            (
                vec![
                    command(
                        ["swapon", "--show", "--bytes", "--raw"],
                        false,
                        "refresh active swap inventory",
                    ),
                    swap_blkid_command(target, "refresh swap signature label and UUID"),
                    swap_inspect_command(
                        target,
                        "inspect modeled swap relationships after refresh",
                    ),
                ],
                vec![
                    "use grow when backing swap capacity changed".to_string(),
                    "use format only when replacing the swap signature is intended".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("zram") => (
            zram_rescan_commands("refresh zram compressed swap inventory"),
            vec![
                "use services.disk-nix.zram to reconcile generated NixOS zramSwap settings"
                    .to_string(),
                "coordinate swapoff before changing live zram size, algorithm, priority, or writeback device"
                    .to_string(),
            ],
            true,
        ),
        Operation::SetProperty if collection == Some("zram") => (
            zram_rescan_commands("inspect zram compressed swap declaration and current inventory"),
            vec![
                "plain zram declarations inspect generated compressed swap state without mutating it"
                    .to_string(),
                "use operation = \"rescan\" for an explicit zram inventory refresh action"
                    .to_string(),
                "use services.disk-nix.zram options to reconcile generated NixOS zramSwap settings"
                    .to_string(),
            ],
            false,
        ),
        Operation::Grow if collection == Some("luks.devices") => {
            let mapper = target.unwrap_or("<mapper>");
            let device = action.context.device.as_deref();
            (
                vec![
                    luks_backing_inspect_command(
                        device,
                        "inspect backing device before resizing the LUKS mapper",
                    ),
                    command(
                        ["cryptsetup", "status", mapper],
                        false,
                        "inspect open LUKS mapper before resize",
                    ),
                    command(
                        ["cryptsetup", "resize", mapper],
                        true,
                        "resize the open LUKS mapping after backing capacity changes",
                    ),
                ],
                vec![
                    "grow the backing partition, LUN, or volume before resizing the mapper"
                        .to_string(),
                    "coordinate dependent LVM and filesystem resizing after cryptsetup resize"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            let desired_size = action.context.desired_size.as_deref();
            let physical_size = action.context.physical_size.as_deref();
            let mut commands = vec![command(
                ["vdo", "status", "--name", target],
                false,
                "inspect VDO logical and physical size before growth",
            )];
            commands.extend(vdo_growth_commands(target, desired_size, physical_size));
            (
                commands,
                vec![
                    "choose logical and physical growth intentionally; they are separate VDO operations"
                        .to_string(),
                    "confirm backing storage capacity before physical VDO growth".to_string(),
                    "review deduplication, compression, and slab utilization before increasing logical size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Start if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            (
                vec![
                    command(
                        ["vdo", "status", "--name", target],
                        false,
                        "inspect VDO volume before start",
                    ),
                    command(
                        ["vdo", "start", "--name", target],
                        true,
                        "start the existing VDO volume after backing storage is present",
                    ),
                ],
                vec![
                    "verify the backing device is present and stable before starting VDO".to_string(),
                    "activate dependent filesystems, LVM layers, or mounts only after VDO status is healthy"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("zvols") => {
            let target = target.unwrap_or("<zvol>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["zfs", "get", "-H", "-p", "volsize", target],
                        false,
                        "inspect current zvol size before growth",
                    ),
                    zvol_set_volsize_command(target, desired_size),
                ],
                vec![
                    "verify pool free space and reservation policy before increasing volsize"
                        .to_string(),
                    "rescan dependent block consumers after zvol growth".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    md_raid_detail_command(
                        target,
                        "inspect MD RAID array health before grow or reshape",
                    ),
                    md_raid_grow_command(target, desired_size),
                    command(
                        ["cat", "/proc/mdstat"],
                        false,
                        "monitor MD RAID reshape, recovery, or resync state",
                    ),
                ],
                vec![
                    "verify backups and redundancy before reshape".to_string(),
                    "do not grow dependent filesystems until mdadm reports the new array size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let mut commands = Vec::new();
            if let Some(target) = target {
                commands.push(command(
                    ["mdadm", "--detail", target],
                    false,
                    "inspect targeted MD RAID array before metadata rescan",
                ));
            }
            commands.extend([
                command(
                    ["mdadm", "--detail", "--scan"],
                    false,
                    "list assembled MD RAID arrays from current metadata",
                ),
                command(
                    ["mdadm", "--examine", "--scan"],
                    false,
                    "scan member devices for MD RAID metadata without assembling arrays",
                ),
                command(
                    ["cat", "/proc/mdstat"],
                    false,
                    "inspect kernel MD RAID status after metadata scan",
                ),
            ]);
            (
                commands,
                vec![
                    "use assemble when reviewed member metadata should activate an array"
                        .to_string(),
                    "verify member event counts before any replacement, grow, or assemble operation"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            (
                vec![
                    multipath_list_command(
                        target,
                        "inspect multipath map paths and size before growth",
                    ),
                    lsscsi_lun_inventory_command(
                        "inspect host-visible SCSI path transport and size before multipath growth",
                    ),
                    multipath_resize_command(target),
                    command(
                        ["multipath", "-r"],
                        true,
                        "reload multipath maps after path rescans",
                    ),
                ],
                vec![
                    "rescan each SCSI path before resizing the multipath map".to_string(),
                    "grow dependent volumes or filesystems only after the map reports the new size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            (
                vec![
                    multipath_list_command(target, "inspect multipath map paths before rescan"),
                    lsscsi_lun_inventory_command(
                        "inspect host-visible SCSI path transport and size before multipath rescan",
                    ),
                    command(
                        ["multipath", "-r"],
                        true,
                        "reload multipath maps after refreshed backing paths",
                    ),
                    multipath_list_command(target, "verify multipath map paths after rescan"),
                    lsscsi_lun_inventory_command(
                        "verify host-visible SCSI path transport and size after multipath rescan",
                    ),
                ],
                vec![
                    "rescan backing SCSI or iSCSI paths before reloading the map".to_string(),
                    "verify the map WWID and every expected path before exposing consumers"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("multipathMaps") => {
            let target = multipath_map_target(action);
            (
                vec![
                    multipath_list_command(target, "inspect multipath map paths before removal"),
                    multipath_flush_map_command(target),
                ],
                vec![
                    "multipath map removal flushes the host map but does not delete target-side data"
                        .to_string(),
                    "unmount filesystems and deactivate LVM, dm, and service consumers before flushing the map"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespaces before rescan",
                    ),
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before rescan"),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(controller, "verify NVMe namespaces after rescan"),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after rescan"),
                ],
                vec![
                    "verify namespace inventory before exposing refreshed devices to consumers"
                        .to_string(),
                    "use grow when controller-side namespace capacity changed".to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            (
                vec![
                    nvme_list_namespaces_command(controller, "inspect NVMe namespaces before rescan"),
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before growth rescan"),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(controller, "verify NVMe namespaces after rescan"),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after growth rescan"),
                ],
                vec![
                    "perform controller-side namespace resize before host rescan".to_string(),
                    "grow dependent partitions, volumes, or filesystems only after the namespace reports the new size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Attach if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            let namespace_id = action.context.namespace_id.as_deref();
            let controllers = action.context.controllers.as_deref();
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespace inventory before attach",
                    ),
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before attach"),
                    nvme_attach_namespace_command(controller, namespace_id, controllers),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(
                        controller,
                        "verify NVMe namespace inventory after attach",
                    ),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after attach"),
                ],
                vec![
                    "attach preserves the namespace and only changes controller visibility"
                        .to_string(),
                    "verify namespace id and controller attachment before exposing consumers"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow if collection == Some("partitions") => {
            let partition_target = partition_target_path(action);
            let disk = action.context.device.as_deref();
            let partition_number = action.context.partition_number.as_deref();
            let desired_end = action
                .context
                .end
                .as_deref()
                .or(action.context.desired_size.as_deref());
            (
                vec![
                    disk_nix_inspect_command(
                        partition_target,
                        "<partition>",
                        "partition path",
                        "inspect partition, consumers, and backing device before growth",
                    ),
                    partition_grow_command(disk, partition_number, desired_end),
                    command(
                        ["partprobe"],
                        true,
                        "ask the kernel to reread partition tables after the geometry change",
                    ),
                    partition_table_reread_command(disk),
                ],
                vec![
                    "confirm the backing disk or LUN has already grown".to_string(),
                    "pause dependent consumers when the kernel cannot reread an active table"
                        .to_string(),
                    "resize LUKS, LVM, and filesystems only after the partition reports the new size"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("disks") || collection == Some("partitions") => {
            let disk = partition_rescan_disk(action);
            (
                vec![
                    disk_nix_inspect_command(
                        disk,
                        "<disk>",
                        "disk path",
                        "inspect disk identity before partition-table rescan",
                    ),
                    partition_probe_command(disk),
                    partition_table_reread_command(disk),
                    disk_parted_machine_list_command(
                        disk,
                        "verify the disk partition table after reread",
                    ),
                ],
                vec![
                    "use grow or create when partition geometry changes are still required"
                        .to_string(),
                    "pause dependent consumers when an active kernel table cannot be reread"
                        .to_string(),
                    "verify stable by-id and by-partuuid paths before growing consumers"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Grow => {
            let target = target.unwrap_or("<target>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect current target state before growth",
                    ),
                    command_with_readiness(
                        ["<grow-storage-object-tool>", target],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["grow tool", "desired size"],
                        "grow the storage object with the target-domain-specific command",
                    ),
                ],
                vec![
                    "select the grow command from the target storage domain and desired size"
                        .to_string(),
                ],
                true,
            )
        }
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
                return (
                    vec![command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect declared storage object state",
                    )],
                    vec!["no property mutation was requested by this declaration".to_string()],
                    false,
                );
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
        Operation::Snapshot => {
            let target = target.unwrap_or("<snapshot>");
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            let snapshot_command = if collection == Some("lvmSnapshots") {
                lvm_snapshot_create_command(
                    target,
                    snapshot,
                    action.context.desired_size.as_deref(),
                )
            } else {
                snapshot_command(
                    collection,
                    target,
                    snapshot,
                    action.context.read_only.unwrap_or(false),
                )
            };
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect snapshot target before creation",
                    ),
                    snapshot_command,
                ],
                Vec::new(),
                true,
            )
        }
        Operation::Create if collection == Some("btrfsSubvolumes") => {
            let target = target.unwrap_or("<btrfs-subvolume-path>");
            (
                vec![
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect target path and parent Btrfs mount before subvolume creation",
                    ),
                    command(
                        ["btrfs", "subvolume", "create", target],
                        true,
                        "create the Btrfs subvolume at the reviewed path",
                    ),
                ],
                vec![
                    "verify the parent path is on the intended Btrfs filesystem".to_string(),
                    "confirm the target path does not already contain data".to_string(),
                    "review qgroup and mount policy before using the new subvolume".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("btrfsSubvolumes") => {
            let target = target.unwrap_or("<btrfs-subvolume-path>");
            let ready = action.context.target.as_deref().is_some();
            let show_command = if ready {
                command(
                    ["btrfs", "subvolume", "show", target],
                    false,
                    "refresh Btrfs subvolume metadata",
                )
            } else {
                command_with_readiness(
                    ["btrfs", "subvolume", "show", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["Btrfs subvolume path"],
                    "refresh Btrfs subvolume metadata after selecting the subvolume path",
                )
            };
            let readonly_command = if ready {
                command(
                    ["btrfs", "property", "get", "-ts", target, "ro"],
                    false,
                    "refresh Btrfs subvolume read-only property",
                )
            } else {
                command_with_readiness(
                    ["btrfs", "property", "get", "-ts", target, "ro"],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["Btrfs subvolume path"],
                    "refresh Btrfs subvolume read-only property after selecting the subvolume path",
                )
            };
            let inspect_command = if ready {
                command(
                    ["disk-nix", "inspect", target],
                    false,
                    "inspect modeled Btrfs subvolume relationships after refresh",
                )
            } else {
                command_with_readiness(
                    ["disk-nix", "inspect", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["Btrfs subvolume path"],
                    "inspect modeled Btrfs subvolume relationships after selecting the subvolume path",
                )
            };
            (
                vec![show_command, readonly_command, inspect_command],
                vec![
                    "subvolume rescan does not change read-only enforcement or namespace layout"
                        .to_string(),
                    "review qgroup and snapshot relationships before later destructive cleanup"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("btrfsQgroups") => {
            let qgroup_id = action.context.name.as_deref().unwrap_or("<qgroupid>");
            let target_path = btrfs_qgroup_target_path(action.context.target.as_deref(), qgroup_id);
            let target = target_path.unwrap_or("<btrfs-filesystem-path>");
            let inspect_command = match target_path {
                Some(target) => command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "inspect Btrfs qgroup inventory before creation",
                ),
                None => command_with_readiness(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "inspect Btrfs qgroups after selecting the mounted filesystem path",
                ),
            };
            let create_command = match target_path {
                Some(target) => command_vec(
                    vec![
                        "btrfs".to_string(),
                        "qgroup".to_string(),
                        "create".to_string(),
                        qgroup_id.to_string(),
                        target.to_string(),
                    ],
                    true,
                    "create the reviewed Btrfs qgroup",
                ),
                None => command_vec_with_readiness(
                    vec![
                        "btrfs".to_string(),
                        "qgroup".to_string(),
                        "create".to_string(),
                        qgroup_id.to_string(),
                        target.to_string(),
                    ],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "create the Btrfs qgroup after selecting the mounted filesystem path",
                ),
            };
            (
                vec![inspect_command, create_command],
                vec![
                    "verify qgroup quota accounting is enabled on the filesystem".to_string(),
                    "select the qgroup id intentionally to avoid hierarchy collisions".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("btrfsQgroups") => {
            let qgroup_id = action.context.name.as_deref().unwrap_or("<qgroupid>");
            let target_path = btrfs_qgroup_target_path(action.context.target.as_deref(), qgroup_id);
            let target = target_path.unwrap_or("<btrfs-filesystem-path>");
            let show_command = match target_path {
                Some(target) => command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "refresh Btrfs qgroup hierarchy, limits, and usage",
                ),
                None => command_with_readiness(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "refresh Btrfs qgroups after selecting the mounted filesystem path",
                ),
            };
            let inspect_command = match target_path {
                Some(target) => command(
                    ["disk-nix", "inspect", target],
                    false,
                    "inspect modeled Btrfs qgroup graph relationships after refresh",
                ),
                None => command_with_readiness(
                    ["disk-nix", "inspect", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "inspect modeled Btrfs qgroup relationships after selecting the mounted filesystem path",
                ),
            };
            (
                vec![show_command, inspect_command],
                vec![
                    format!("review qgroup {qgroup_id} usage before limit or removal changes"),
                    "qgroup rescan does not change quota enforcement or delete policy".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            let desired_size = action.context.desired_size.as_deref();
            let namespace_id = action.context.namespace_id.as_deref();
            let controllers = action.context.controllers.as_deref();
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespace inventory before creation",
                    ),
                    nvme_create_namespace_command(controller, desired_size),
                    nvme_attach_namespace_command(controller, namespace_id, controllers),
                    nvme_namespace_rescan_command(controller),
                ],
                vec![
                    "nvme create-ns returns the namespace id; declare namespaceId before attach can be executable"
                        .to_string(),
                    "verify controller and namespace capacity before exposing consumers".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("pools") => {
            let target = action
                .context
                .name
                .as_deref()
                .or(target)
                .unwrap_or("<zfs-pool>");
            let device = action.context.device.as_deref();
            let devices = pool_create_devices(device, &action.context.devices);
            let mut commands = zfs_pool_preflight_commands(&devices);
            commands.push(zfs_pool_create_command(
                target,
                &devices,
                &action.context.property_assignments,
            ));
            (
                commands,
                vec![
                    "verify every vdev device is empty or fully backed up before pool creation"
                        .to_string(),
                    "choose redundancy, ashift, feature, and autotrim policy before creating datasets"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            (
                vec![
                    command(
                        ["cat", "/proc/mdstat"],
                        false,
                        "inspect existing MD RAID state before array creation",
                    ),
                    md_raid_create_command(
                        target,
                        action.context.level.as_deref(),
                        action.context.options.as_deref(),
                        &action.context.devices,
                    ),
                ],
                vec![
                    "verify every member device is empty or fully backed up before array creation"
                        .to_string(),
                    "choose metadata, bitmap, and spare policy before creating production arrays"
                        .to_string(),
                    "monitor /proc/mdstat until initial sync completes".to_string(),
                ],
                true,
            )
        }
        Operation::Assemble if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            (
                vec![
                    command(
                        ["cat", "/proc/mdstat"],
                        false,
                        "inspect existing MD RAID state before array assembly",
                    ),
                    md_raid_assemble_command(target, &action.context.devices),
                ],
                vec![
                    "verify member event counts and array UUID before assembly".to_string(),
                    "activate filesystems and mappings only after mdadm reports expected health"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Stop if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            (
                vec![
                    md_raid_detail_command(target, "inspect MD RAID array before stopping"),
                    md_raid_stop_command(target),
                ],
                vec![
                    "unmount filesystems and deactivate mappings before stopping the array"
                        .to_string(),
                    "preserve member devices for later mdadm assemble".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("zvols") => {
            let target = target.unwrap_or("<zvol>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["zpool", "list", "-H", "-p"],
                        false,
                        "inspect ZFS pool free space before creating the zvol",
                    ),
                    zvol_create_command(target, desired_size, &action.context.property_assignments),
                ],
                vec![
                    "decide sparse versus reserved allocation before creation".to_string(),
                    "expose the zvol to guests or LUN exports only after verification".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("datasets") => {
            let target = target.unwrap_or("<zfs-dataset>");
            (
                vec![
                    command(
                        ["zpool", "list", "-H", "-p"],
                        false,
                        "inspect ZFS pool free space before creating the dataset",
                    ),
                    zfs_dataset_create_command(target, &action.context.property_assignments),
                ],
                vec![
                    "review inherited mountpoint, quota, reservation, and encryption properties"
                        .to_string(),
                    "set required properties before exposing the dataset to consumers".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("datasets") => {
            let target = target.unwrap_or("<zfs-dataset>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", "-t", "filesystem", target],
                        false,
                        "refresh ZFS dataset inventory, mountpoint, and usage",
                    ),
                    command(
                        [
                            "zfs",
                            "get",
                            "-H",
                            "-p",
                            "-o",
                            "property,value,source",
                            "all",
                            target,
                        ],
                        false,
                        "refresh ZFS dataset property sources",
                    ),
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect modeled ZFS dataset relationships after refresh",
                    ),
                ],
                vec![
                    "dataset rescan does not change mountpoints, quotas, or reservations"
                        .to_string(),
                    "use property updates only after reviewing inherited policy".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("zvols") => {
            let target = target.unwrap_or("<zvol>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", "-t", "volume", target],
                        false,
                        "refresh zvol inventory, volsize, and usage",
                    ),
                    command(
                        [
                            "zfs",
                            "get",
                            "-H",
                            "-p",
                            "-o",
                            "property,value,source",
                            "all",
                            target,
                        ],
                        false,
                        "refresh zvol property sources",
                    ),
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect modeled zvol block relationships after refresh",
                    ),
                ],
                vec![
                    "zvol rescan does not change volsize, reservations, or consumers".to_string(),
                    "use grow only after reviewing pool capacity and downstream consumers"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            (
                vec![
                    command(
                        ["vdo", "status", "--name", target],
                        false,
                        "refresh VDO volume status and configuration",
                    ),
                    command(
                        ["vdostats", "--human-readable", target],
                        false,
                        "refresh VDO runtime capacity, utilization, and savings counters",
                    ),
                    command(
                        ["disk-nix", "inspect", target],
                        false,
                        "inspect VDO graph node and backing relationships after status refresh",
                    ),
                ],
                vec![
                    "use grow when logical or physical capacity must change".to_string(),
                    "use start or stop only when activation state must change".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("volumes") => {
            let target = target.unwrap_or("<logical-volume>");
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json"],
                        false,
                        "inspect volume group free space before creating the logical volume",
                    ),
                    lvm_logical_volume_create_command(target, desired_size),
                ],
                vec![
                    "verify the target volume group has enough free extents".to_string(),
                    "create filesystems, LUKS mappings, or exports only after the LV appears"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            let device = action.context.device.as_deref();
            let desired_size = action.context.desired_size.as_deref();
            (
                vec![
                    vdo_backing_inspect_command(device),
                    vdo_create_command(target, device, desired_size),
                ],
                vec![
                    "verify the backing device has no signatures or data that must be preserved"
                        .to_string(),
                    "select logical size, deduplication, and compression policy before exposing the VDO device"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let device = action.context.device.as_deref();
            (
                vec![
                    command(
                        ["pvs", "--reportformat", "json"],
                        false,
                        "inspect physical volumes before creating the volume group",
                    ),
                    lvm_volume_group_create_command(target, device),
                ],
                vec![
                    "verify the physical volume path is stable and intentionally selected"
                        .to_string(),
                    "create logical volumes only after the VG appears and free extents are reviewed"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("loopDevices") => {
            let target = target.unwrap_or("<loop-device>");
            let backing = action.context.device.as_deref();
            (
                vec![loop_device_create_command(target, backing)],
                vec![
                    "verify the backing file or block device is the intended source".to_string(),
                    "persist the mapping declaratively when it must survive reboot".to_string(),
                ],
                true,
            )
        }
        Operation::Create | Operation::Export if collection == Some("exports") => {
            let target = export_target_path(action);
            (
                vec![nfs_export_create_command(
                    target,
                    action.context.client.as_deref(),
                    action.context.options.as_deref(),
                )],
                vec![
                    "verify the local export path exists and has intended ownership".to_string(),
                    "prefer restrictive client selectors and read-only options before write access"
                        .to_string(),
                    "persist long-lived exports declaratively through NixOS configuration"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("exports") => {
            let target = export_target_path(action);
            let inspect_target = target.unwrap_or("<export-path>");
            let inspect_command = match target {
                Some(target) => command(
                    ["disk-nix", "inspect", target],
                    false,
                    "inspect modeled NFS export relationships after refresh",
                ),
                None => command_with_readiness(
                    ["disk-nix", "inspect", inspect_target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["NFS export path"],
                    "inspect modeled NFS export relationships after selecting the export path",
                ),
            };
            (
                vec![
                    command(
                        ["exportfs", "-v"],
                        false,
                        "refresh NFS export inventory and client options",
                    ),
                    inspect_command,
                ],
                vec![
                    "export rescan does not reload exports or change client access".to_string(),
                    "use option updates only after reviewing active client visibility".to_string(),
                ],
                true,
            )
        }
        Operation::Create | Operation::Mount if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            (
                vec![nfs_mount_create_command(
                    action.context.device.as_deref(),
                    mountpoint,
                    action.context.fs_type.as_deref(),
                    action.context.options.as_deref(),
                )],
                vec![
                    "verify the NFS server, export permissions, and network path before mounting"
                        .to_string(),
                    "persist long-lived mounts through the NixOS fileSystems entry".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            let inspect_target = mountpoint.unwrap_or("<mountpoint>");
            let inspect_command = match mountpoint {
                Some(mountpoint) => command(
                    ["disk-nix", "inspect", mountpoint],
                    false,
                    "inspect modeled NFS mount relationships after refresh",
                ),
                None => command_with_readiness(
                    ["disk-nix", "inspect", inspect_target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mountpoint path"],
                    "inspect modeled NFS mount relationships after selecting the mountpoint",
                ),
            };
            (
                vec![
                    nfs_mount_findmnt_command(mountpoint),
                    nfs_mount_stats_command(mountpoint),
                    inspect_command,
                ],
                vec![
                    "mount rescan does not remount, unmount, or change remote data".to_string(),
                    "use remount only after reviewing active services and desired options"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Remount if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            (
                vec![
                    nfs_mount_findmnt_command(mountpoint),
                    nfs_mount_remount_command(mountpoint, action.context.options.as_deref()),
                ],
                vec![
                    "review active services before changing NFS mount options".to_string(),
                    "persist the final options through the NixOS fileSystems entry".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("disks") => {
            let disk = disk_target_path(action);
            let label = action.context.partition_type.as_deref().unwrap_or("gpt");
            if label == "zfs" {
                return (
                    vec![
                        disk_nix_inspect_command(
                            disk,
                            "<disk>",
                            "disk path",
                            "inspect disk identity, signatures, and existing consumers before raw ZFS initialization",
                        ),
                        disk_wipe_signatures_command(disk),
                        partition_probe_command(disk),
                    ],
                    vec![
                        "raw ZFS disks do not receive a parted partition table".to_string(),
                        "zpool create writes ZFS labels to the reviewed whole-disk device"
                            .to_string(),
                        "prefer importing an existing pool when the disk already contains ZFS labels"
                            .to_string(),
                    ],
                    true,
                );
            }
            (
                vec![
                    disk_nix_inspect_command(
                        disk,
                        "<disk>",
                        "disk path",
                        "inspect disk identity, signatures, and existing consumers before initialization",
                    ),
                    disk_create_label_command(disk, label),
                    partition_probe_command(disk),
                    disk_parted_machine_list_command(
                        disk,
                        "verify the disk reports the reviewed partition table label",
                    ),
                ],
                vec![
                    "creating a partition table can hide existing signatures and partitions"
                        .to_string(),
                    "prefer importing or preserving existing metadata when the disk is not empty"
                        .to_string(),
                    "create partitions only after the initialized disk is re-probed".to_string(),
                ],
                true,
            )
        }
        Operation::Create if collection == Some("partitions") => {
            let partition_target = partition_target_path(action);
            let disk = action.context.device.as_deref();
            let start = action.context.start.as_deref();
            let end = action.context.end.as_deref();
            let partition_type = action.context.partition_type.as_deref();
            let mut commands = vec![disk_nix_inspect_command(
                disk,
                "<disk>",
                "disk path",
                "inspect disk identity and existing partition table before creation",
            )];
            if disk.is_some_and(|disk| disk.starts_with("/dev/md/"))
                && action.context.partition_number.as_deref() == Some("1")
            {
                commands.push(disk_create_label_command(disk, "gpt"));
            }
            commands.extend([
                partition_create_command(disk, partition_type, start, end),
                partition_probe_command(disk),
                partition_table_reread_command(disk),
                disk_nix_inspect_command(
                    partition_target,
                    "<partition>",
                    "partition path",
                    "verify the new partition node before creating higher layers",
                ),
            ]);
            (
                commands,
                vec![
                    "verify the selected disk path is stable and matches the intended hardware"
                        .to_string(),
                    "verify the start and end offsets are inside known-free space".to_string(),
                    "format or map the new partition only after it appears by stable identity"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Format if collection == Some("swaps") => {
            let target = swap_target_path(action);
            (
                vec![
                    disk_nix_inspect_command(
                        target,
                        "<swap>",
                        "swap target path",
                        "inspect target before creating a swap signature",
                    ),
                    swapoff_best_effort_command(
                        target,
                        "disable active swap before replacing its signature",
                    ),
                    swap_command("mkswap", target, "create a swap signature on the target"),
                ],
                vec![
                    "verify the target does not contain data that must be preserved".to_string(),
                    "confirm NixOS swapDevices points at a stable device identity".to_string(),
                ],
                true,
            )
        }
        Operation::Format if collection == Some("luks.devices") => {
            let mapper = target.unwrap_or("<mapper>");
            let device = action.context.device.as_deref();
            (
                vec![
                    luks_backing_inspect_command(
                        device,
                        "inspect target before creating a LUKS container",
                    ),
                    luks_format_command(device),
                    luks_open_command(
                        device,
                        mapper,
                        "open the newly created LUKS container with the desired mapper name",
                    ),
                ],
                vec![
                    "verify header backups and key enrollment policy before formatting".to_string(),
                    "create filesystems or LVM layers only after the mapper is open".to_string(),
                ],
                true,
            )
        }
        Operation::Create | Operation::Open if collection == Some("luks.devices") => {
            let mapper = target.unwrap_or("<mapper>");
            let device = action.context.device.as_deref();
            (
                vec![
                    luks_backing_inspect_command(
                        device,
                        "inspect existing LUKS container before opening",
                    ),
                    luks_is_luks_command(device),
                    luks_open_command(
                        device,
                        mapper,
                        "open the existing LUKS container with the desired mapper name",
                    ),
                ],
                vec![
                    "verify the backing device identity before entering credentials".to_string(),
                    "keep formatting as a separate explicit action when data must be replaced"
                        .to_string(),
                    "create filesystems or LVM layers only after the mapper is open".to_string(),
                ],
                true,
            )
        }
        Operation::Create | Operation::AddKey if collection == Some("luksKeyslots") => {
            let device = luks_keyslot_device(action);
            let key_slot = luks_keyslot_id(action);
            let new_key_file = luks_new_key_file(action);
            (
                vec![
                    luks_dump_command(device, "inspect LUKS header before adding keyslot"),
                    luks_add_key_command(device, key_slot, new_key_file),
                ],
                vec![
                    "back up the LUKS header before enrolling new key material".to_string(),
                    "test the new keyslot before removing any old recovery key".to_string(),
                ],
                true,
            )
        }
        Operation::Create | Operation::ImportToken if collection == Some("luksTokens") => {
            let device = luks_token_device(action);
            let token_id = luks_token_id(action);
            let token_file = luks_token_file(action);
            (
                vec![
                    luks_dump_command(device, "inspect LUKS header before importing token"),
                    luks_token_import_command(device, token_id, token_file),
                ],
                vec![
                    "back up the LUKS header before importing token metadata".to_string(),
                    "test the token unlock path before removing any older token".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::Close if collection == Some("luks.devices") => {
            let mapper = target.unwrap_or("<mapper>");
            (
                vec![
                    command(
                        ["cryptsetup", "status", mapper],
                        false,
                        "inspect open LUKS mapper before close",
                    ),
                    command(
                        ["cryptsetup", "close", mapper],
                        true,
                        "close the reviewed LUKS mapper without erasing backing data",
                    ),
                ],
                vec![
                    "unmount filesystems and deactivate LVM volumes before closing the mapper"
                        .to_string(),
                    "verify no services still depend on the mapper path".to_string(),
                    "keep the backing LUKS header intact for later reopen".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::RemoveKey if collection == Some("luksKeyslots") => {
            let device = luks_keyslot_device(action);
            let key_slot = luks_keyslot_id(action);
            (
                vec![
                    luks_dump_command(device, "inspect LUKS keyslots before removal"),
                    luks_kill_slot_command(device, key_slot),
                ],
                vec![
                    "verify another key, token, or recovery passphrase unlocks the device first"
                        .to_string(),
                    "keep a LUKS header backup until post-removal unlock testing passes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy | Operation::RemoveToken if collection == Some("luksTokens") => {
            let device = luks_token_device(action);
            let token_id = luks_token_id(action);
            (
                vec![
                    luks_dump_command(device, "inspect LUKS tokens before removal"),
                    luks_token_remove_command(device, token_id),
                ],
                vec![
                    "verify another keyslot, token, or recovery passphrase unlocks the device first"
                        .to_string(),
                    "keep a LUKS header backup until post-removal unlock testing passes".to_string(),
                ],
                true,
            )
        }
        Operation::Create => (
            vec![command_with_readiness(
                ["<create-storage-object-tool>", "<target>"],
                true,
                CommandReadiness::NeedsDomainImplementation,
                ["create tool", "target"],
                "create the requested storage object",
            )],
            vec![
                "creation commands require target-kind-specific arguments from the desired spec"
                    .to_string(),
            ],
            true,
        ),
        Operation::Destroy if collection == Some("btrfsSubvolumes") => {
            let target = target.unwrap_or("<btrfs-subvolume-path>");
            (
                vec![
                    command(
                        ["btrfs", "subvolume", "show", target],
                        false,
                        "inspect Btrfs subvolume metadata before deletion",
                    ),
                    command(
                        ["btrfs", "subvolume", "delete", target],
                        true,
                        "delete the reviewed Btrfs subvolume",
                    ),
                ],
                vec![
                    "take a read-only snapshot before deletion when data may be needed".to_string(),
                    "unmount or redirect consumers before deleting the subvolume".to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("btrfsSubvolumes") => {
            let target = target.unwrap_or("<btrfs-subvolume-path>");
            let rename_to = action
                .context
                .rename_to
                .as_deref()
                .unwrap_or("<new-btrfs-subvolume-path>");
            (
                vec![
                    command(
                        ["btrfs", "subvolume", "show", target],
                        false,
                        "inspect Btrfs subvolume before rename",
                    ),
                    command(
                        ["mv", "--", target, rename_to],
                        true,
                        "rename the reviewed Btrfs subvolume path",
                    ),
                ],
                vec![
                    "update mounts, send/receive jobs, qgroups, and snapshots after rename"
                        .to_string(),
                    "validate consumers on the renamed subvolume before deleting the old path"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("btrfsQgroups") => {
            let qgroup_id = action.context.name.as_deref().unwrap_or("<qgroupid>");
            let target_path = btrfs_qgroup_target_path(action.context.target.as_deref(), qgroup_id);
            let target = target_path.unwrap_or("<btrfs-filesystem-path>");
            let inspect_command = match target_path {
                Some(target) => command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "inspect Btrfs qgroup inventory before destruction",
                ),
                None => command_with_readiness(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "inspect Btrfs qgroups after selecting the mounted filesystem path",
                ),
            };
            let destroy_command = match target_path {
                Some(target) => command_vec(
                    vec![
                        "btrfs".to_string(),
                        "qgroup".to_string(),
                        "destroy".to_string(),
                        qgroup_id.to_string(),
                        target.to_string(),
                    ],
                    true,
                    "destroy the reviewed Btrfs qgroup",
                ),
                None => command_vec_with_readiness(
                    vec![
                        "btrfs".to_string(),
                        "qgroup".to_string(),
                        "destroy".to_string(),
                        qgroup_id.to_string(),
                        target.to_string(),
                    ],
                    true,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mounted Btrfs filesystem path"],
                    "destroy the Btrfs qgroup after selecting the mounted filesystem path",
                ),
            };
            (
                vec![inspect_command, destroy_command],
                vec![
                    "verify no subvolume still depends on the qgroup limit".to_string(),
                    "preserve qgroup accounting policy elsewhere before deleting the qgroup"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("zvols") => {
            let target = target.unwrap_or("<zvol>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", "-t", "volume", target],
                        false,
                        "inspect zvol metadata before destruction",
                    ),
                    command(
                        ["zfs", "destroy", target],
                        true,
                        "destroy the reviewed zvol after consumers are detached",
                    ),
                ],
                vec![
                    "take a snapshot or clone before destruction when rollback is required"
                        .to_string(),
                    "detach LUN, VM, or filesystem consumers before destroying the zvol"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("pools") => {
            let target = zfs_pool_command_target(action, target);
            (
                vec![
                    command(
                        ["zpool", "status", "-P", target],
                        false,
                        "inspect pool health and dependent vdevs before destruction",
                    ),
                    command(
                        ["zpool", "destroy", target],
                        true,
                        "destroy the reviewed ZFS pool after datasets and consumers are migrated",
                    ),
                ],
                vec![
                    "take recursive snapshots or verified backups before destroying the pool"
                        .to_string(),
                    "export the pool instead of destroying it when moving it to another host"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Import if collection == Some("pools") => {
            let target = zfs_pool_command_target(action, target);
            (
                vec![
                    command(
                        ["zpool", "import"],
                        false,
                        "inspect importable ZFS pools before import",
                    ),
                    zfs_pool_import_command(target, action.context.read_only.unwrap_or(false)),
                ],
                vec![
                    "verify the pool identity, hostid, cachefile, mountpoints, and encryption keys before import"
                        .to_string(),
                    "use readOnly=true first when validating a moved or recovered pool".to_string(),
                ],
                true,
            )
        }
        Operation::Export if collection == Some("pools") => {
            let target = zfs_pool_command_target(action, target);
            (
                vec![
                    command(
                        ["zpool", "status", "-P", target],
                        false,
                        "inspect pool health and active consumers before export",
                    ),
                    command(
                        ["zpool", "export", target],
                        true,
                        "export the reviewed ZFS pool without deleting data",
                    ),
                ],
                vec![
                    "stop mount, share, LUN, VM, and service consumers before export".to_string(),
                    "export instead of destroying a pool that will be moved or recovered elsewhere"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("datasets") => {
            let target = target.unwrap_or("<zfs-dataset>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", "-r", target],
                        false,
                        "inspect dataset descendants before destruction",
                    ),
                    command(
                        ["zfs", "destroy", target],
                        true,
                        "destroy the reviewed ZFS dataset after snapshots and consumers are handled",
                    ),
                ],
                vec![
                    "take a recursive snapshot or clone before destruction when rollback is required"
                        .to_string(),
                    "unmount dependents and review child datasets before destroying the dataset"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("datasets") || collection == Some("zvols") => {
            let target = target.unwrap_or("<zfs-dataset>");
            let rename_to = action
                .context
                .rename_to
                .as_deref()
                .unwrap_or("<new-zfs-name>");
            (
                vec![
                    command(
                        ["zfs", "list", "-H", "-p", target],
                        false,
                        "inspect ZFS object before rename",
                    ),
                    command(
                        ["zfs", "rename", target, rename_to],
                        true,
                        "rename the reviewed ZFS dataset or zvol",
                    ),
                ],
                vec![
                    "update mountpoints, shares, LUN mappings, and dependent services to the new name"
                        .to_string(),
                    "validate consumers on the renamed object before destroying any old path"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Promote if collection == Some("datasets") || collection == Some("zvols") => {
            let target = target.unwrap_or("<zfs-clone>");
            (
                vec![
                    command(
                        ["zfs", "get", "-H", "-o", "value", "origin", target],
                        false,
                        "inspect ZFS clone origin before promotion",
                    ),
                    command(
                        ["zfs", "promote", target],
                        true,
                        "promote the reviewed ZFS clone",
                    ),
                ],
                vec![
                    "promotion changes clone dependency ownership; review dependent snapshots first"
                        .to_string(),
                    "validate consumers on the promoted clone before destroying or renaming the origin"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or("<snapshot>");
            let source = action
                .context
                .target
                .as_deref()
                .unwrap_or("<snapshot-source>");
            if is_zfs_snapshot_name(snapshot) {
                (
                    vec![
                        command(
                            ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                            false,
                            "inspect ZFS snapshot before destruction",
                        ),
                        command(
                            ["zfs", "destroy", snapshot],
                            true,
                            "destroy the reviewed ZFS snapshot recovery point",
                        ),
                    ],
                    vec![
                        "verify the snapshot is no longer needed as a recovery point".to_string(),
                        "hold, rename, clone, or replicate the snapshot before destruction when retention is uncertain"
                            .to_string(),
                    ],
                    true,
                )
            } else if is_btrfs_snapshot_pair(source, snapshot) {
                (
                    vec![
                        command(
                            ["btrfs", "subvolume", "show", snapshot],
                            false,
                            "inspect Btrfs snapshot subvolume before deletion",
                        ),
                        command(
                            ["btrfs", "subvolume", "delete", snapshot],
                            true,
                            "delete the reviewed Btrfs snapshot subvolume",
                        ),
                    ],
                    vec![
                        "verify the snapshot is no longer needed as a recovery point".to_string(),
                        "keep or clone the read-only snapshot before deletion when retention is uncertain"
                            .to_string(),
                    ],
                    true,
                )
            } else {
                (
                    vec![command_with_readiness(
                        ["<snapshot-destroy-tool>", source, snapshot],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["snapshot destroy tool"],
                        "destroy the snapshot with zfs, btrfs, lvm, or the target-specific tool",
                    )],
                    vec![
                        "snapshot destruction command is only rendered for unambiguous ZFS names or Btrfs absolute paths"
                            .to_string(),
                    ],
                    true,
                )
            }
        }
        Operation::Rename if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or("<snapshot>");
            let rename_to = action
                .context
                .rename_to
                .as_deref()
                .unwrap_or("<new-snapshot-name>");
            if is_zfs_snapshot_name(snapshot) {
                (
                    vec![
                        command(
                            ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                            false,
                            "inspect ZFS snapshot before rename",
                        ),
                        command(
                            ["zfs", "rename", snapshot, rename_to],
                            true,
                            "rename the reviewed ZFS snapshot recovery point",
                        ),
                    ],
                    vec![
                        "update retention, replication, and rollback references to the new snapshot name"
                            .to_string(),
                    ],
                    true,
                )
            } else if snapshot.starts_with('/') {
                (
                    vec![
                        command(
                            ["btrfs", "subvolume", "show", snapshot],
                            false,
                            "inspect Btrfs snapshot subvolume before rename",
                        ),
                        command(
                            ["mv", "--", snapshot, rename_to],
                            true,
                            "rename the reviewed Btrfs snapshot subvolume path",
                        ),
                    ],
                    vec![
                        "update retention and restore references to the renamed snapshot path"
                            .to_string(),
                    ],
                    true,
                )
            } else {
                (
                    vec![command_with_readiness(
                        ["<snapshot-rename-tool>", snapshot, rename_to],
                        true,
                        CommandReadiness::NeedsDomainImplementation,
                        ["ZFS snapshot name or Btrfs snapshot path"],
                        "rename the snapshot after selecting the target-specific snapshot tool",
                    )],
                    vec![
                        "snapshot rename command is only rendered for unambiguous ZFS snapshot names or Btrfs absolute paths"
                            .to_string(),
                    ],
                    true,
                )
            }
        }
        Operation::Destroy if collection == Some("lvmSnapshots") => {
            let target = target.unwrap_or("<lvm-snapshot>");
            (
                vec![
                    command(
                        ["lvs", "--reportformat", "json", target],
                        false,
                        "inspect LVM snapshot before removal",
                    ),
                    command(
                        ["lvremove", "--yes", target],
                        true,
                        "remove the reviewed LVM snapshot",
                    ),
                ],
                vec![
                    "verify the snapshot is no longer needed as a recovery point".to_string(),
                    "prefer a fresh snapshot or backup before deleting old snapshots".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("volumes") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(target, None, "inspect logical volume before removal"),
                    lvm_lvremove_command(
                        target,
                        "<logical-volume>",
                        "target in volume-group/logical-volume form",
                        "remove the reviewed logical volume after backups and consumers are verified",
                    ),
                ],
                vec![
                    "snapshot or migrate data before removing the logical volume".to_string(),
                    "unmount filesystems and deactivate dependent mappings before lvremove"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("volumes") || collection == Some("thinPools") => {
            let target = lvm_volume_target_path(target);
            let rename_to = action.context.rename_to.as_deref();
            (
                vec![
                    lvm_lvs_report_command(target, None, "inspect logical volume before rename"),
                    lvm_lvrename_command(
                        target,
                        rename_to,
                        "<logical-volume>",
                        "target in volume-group/logical-volume form",
                        "new logical volume name or path",
                        "rename the reviewed logical volume",
                    ),
                ],
                vec![
                    "update filesystems, crypttab, mounts, LUN exports, and services after rename"
                        .to_string(),
                    "keep the old declaration out of destructive mode until consumers are validated"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Activate | Operation::Deactivate
            if collection == Some("volumes")
                || collection == Some("thinPools")
                || collection == Some("lvmSnapshots") =>
        {
            let target = lvm_volume_target_path(target);
            let (flag, verb, placeholder, input) = match collection {
                Some("thinPools") => (
                    "y",
                    "activate",
                    "<thin-pool>",
                    "target in volume-group/thin-pool form",
                ),
                Some("lvmSnapshots") => (
                    "y",
                    "activate",
                    "<lvm-snapshot>",
                    "target in volume-group/snapshot form",
                ),
                _ => (
                    "y",
                    "activate",
                    "<logical-volume>",
                    "target in volume-group/logical-volume form",
                ),
            };
            let (flag, verb) = if action.operation == Operation::Deactivate {
                ("n", "deactivate")
            } else {
                (flag, verb)
            };
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        None,
                        "inspect logical volume before activation change",
                    ),
                    lvm_lvchange_activate_command(target, flag, placeholder, input),
                ],
                vec![
                    format!(
                        "{verb} only after filesystem, mapping, mount, and service consumers are reviewed"
                    ),
                    "activation state changes do not create or delete LV data".to_string(),
                ],
                true,
            )
        }
        Operation::Rescan if collection == Some("lvmSnapshots") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,origin,lv_attr,data_percent,metadata_percent,lv_size"),
                        "refresh LVM snapshot origin, attributes, and COW usage",
                    ),
                    command(
                        ["disk-nix", "inspect", target.unwrap_or("<lvm-snapshot>")],
                        false,
                        "inspect modeled LVM snapshot graph relationships after status refresh",
                    ),
                ],
                vec![
                    "use rollback only after reviewing origin and snapshot state".to_string(),
                    "activate the snapshot for recovery inspection before destructive removal"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("thinPools") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_attr,data_percent,metadata_percent"),
                        "inspect thin pool before removal",
                    ),
                    lvm_lvremove_command(
                        target,
                        "<thin-pool>",
                        "target in volume-group/thin-pool form",
                        "remove the reviewed thin pool after thin volumes and consumers are migrated",
                    ),
                ],
                vec![
                    "migrate or remove thin volumes before removing the thin pool".to_string(),
                    "unmount filesystems and deactivate mappings that depend on thin volumes"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before removal",
                    ),
                    command(
                        ["vgremove", "--yes", target],
                        true,
                        "remove the reviewed LVM volume group after all consumers are migrated",
                    ),
                ],
                vec![
                    "remove or migrate logical volumes before removing the volume group"
                        .to_string(),
                    "verify no filesystems, mappings, or services still reference the VG"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Import if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            (
                vec![
                    command(
                        ["pvs", "--reportformat", "json"],
                        false,
                        "inspect physical volumes and exported VG metadata before import",
                    ),
                    command(
                        ["vgimport", target],
                        true,
                        "import the reviewed LVM volume group without recreating it",
                    ),
                ],
                vec![
                    "verify PV identities, VG UUID, and metadata backups before vgimport"
                        .to_string(),
                    "activate logical volumes and mount consumers only after the VG is verified"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Export if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before export",
                    ),
                    command(
                        ["vgexport", target],
                        true,
                        "export the reviewed LVM volume group without deleting data",
                    ),
                ],
                vec![
                    "deactivate logical volumes and stop mount, mapping, and service consumers before vgexport"
                        .to_string(),
                    "export instead of removing a VG that will be moved to another host"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Rename if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let rename_to = action
                .context
                .rename_to
                .as_deref()
                .unwrap_or("<new-volume-group>");
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before rename",
                    ),
                    command(
                        ["vgrename", target, rename_to],
                        true,
                        "rename the reviewed volume group",
                    ),
                ],
                vec![
                    "update every LV path, initrd reference, mount, and service before reboot"
                        .to_string(),
                    "validate boot and activation with the renamed volume group before cleanup"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Activate | Operation::Deactivate if collection == Some("volumeGroups") => {
            let target = target.unwrap_or("<volume-group>");
            let (flag, verb) = if action.operation == Operation::Deactivate {
                ("n", "deactivate")
            } else {
                ("y", "activate")
            };
            (
                vec![
                    command(
                        ["vgs", "--reportformat", "json", target],
                        false,
                        "inspect volume group before activation change",
                    ),
                    command(
                        ["vgchange", "--activate", flag, target],
                        true,
                        if flag == "y" {
                            "activate the reviewed LVM volume group"
                        } else {
                            "deactivate the reviewed LVM volume group without deleting data"
                        },
                    ),
                ],
                vec![
                    format!(
                        "{verb} the VG only after PV membership and dependent consumers are reviewed"
                    ),
                    "volume group activation changes do not create or remove VG metadata"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("physicalVolumes") => {
            let target = lvm_physical_volume_target(action);
            (
                vec![
                    lvm_physical_volume_inspect_command(target),
                    lvm_physical_volume_remove_command(target),
                ],
                vec![
                    "run pvmove and vgreduce before pvremove when the PV is in a VG".to_string(),
                    "keep the device available for recovery until backups are verified".to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("lvmCaches") => {
            let target = lvm_volume_target_path(target);
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent"),
                        "inspect LVM cache dirty state before removal",
                    ),
                    lvm_cache_uncache_command(target),
                ],
                vec![
                    "switch writeback caches to writethrough before removing cache state"
                        .to_string(),
                    "verify the origin LV after lvconvert --uncache before removing cache media"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            (
                vec![
                    command(
                        ["vdo", "status", "--name", target],
                        false,
                        "inspect VDO volume before removal",
                    ),
                    command(
                        ["vdo", "remove", "--name", target],
                        true,
                        "remove the reviewed VDO volume after consumers are migrated",
                    ),
                ],
                vec![
                    "migrate data away from the VDO device before removal".to_string(),
                    "unmount filesystems and deactivate mappings that reference the VDO device"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Stop if collection == Some("vdoVolumes") => {
            let target = target.unwrap_or("<vdo-volume>");
            (
                vec![
                    command(
                        ["vdo", "status", "--name", target],
                        false,
                        "inspect VDO volume before stop",
                    ),
                    command(
                        ["vdo", "stop", "--name", target],
                        true,
                        "stop the existing VDO volume after consumers are inactive",
                    ),
                ],
                vec![
                    "unmount filesystems and deactivate mappings that reference the VDO device"
                        .to_string(),
                    "prefer stop over remove when preserving VDO metadata for later restart"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            let namespace_id = action.context.namespace_id.as_deref();
            let controllers = action.context.controllers.as_deref();
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespace inventory before deletion",
                    ),
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before deletion"),
                    nvme_detach_namespace_command(controller, namespace_id, controllers),
                    nvme_delete_namespace_command(controller, namespace_id),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after deletion"),
                ],
                vec![
                    "detach namespace consumers and migrate data before delete-ns".to_string(),
                    "prefer detach without delete when target-side namespace data must remain"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Detach if collection == Some("nvmeNamespaces") => {
            let controller = nvme_controller_target(action);
            let namespace_id = action.context.namespace_id.as_deref();
            let controllers = action.context.controllers.as_deref();
            (
                vec![
                    nvme_list_namespaces_command(
                        controller,
                        "inspect NVMe namespace inventory before detach",
                    ),
                    nvme_list_subsystems_command("inspect NVMe subsystem paths before detach"),
                    nvme_detach_namespace_command(controller, namespace_id, controllers),
                    nvme_namespace_rescan_command(controller),
                    nvme_list_namespaces_command(
                        controller,
                        "verify NVMe namespace inventory after detach",
                    ),
                    nvme_list_subsystems_command("verify NVMe subsystem paths after detach"),
                ],
                vec![
                    "detach removes controller access without deleting the namespace".to_string(),
                    "unmount filesystems and deactivate dependent mappings before detach"
                        .to_string(),
                ],
                true,
            )
        }
        Operation::Destroy if collection == Some("loopDevices") => {
            let target = loop_device_target_path(action);
            (
                vec![
                    loop_device_list_command(
                        target,
                        "inspect loop device and backing file before detach",
                    ),
                    loop_device_detach_command(target),
                ],
                vec![
                    "unmount filesystems and deactivate mappings before detach".to_string(),
                    "verify the backing file remains available after detach".to_string(),
                ],
                true,
            )
        }
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
        Operation::Format
        | Operation::Shrink
        | Operation::Check
        | Operation::Repair
        | Operation::Clone
        | Operation::Promote
        | Operation::Import
        | Operation::Export
        | Operation::Unexport
        | Operation::Attach
        | Operation::Detach
        | Operation::Activate
        | Operation::Deactivate
        | Operation::Assemble
        | Operation::Start
        | Operation::Stop
        | Operation::Login
        | Operation::Logout
        | Operation::Open
        | Operation::Close
        | Operation::Mount
        | Operation::Unmount
        | Operation::Remount
        | Operation::Rename
        | Operation::Rescan
        | Operation::AddKey
        | Operation::RemoveKey
        | Operation::ImportToken
        | Operation::RemoveToken
        | Operation::RemoveDevice
        | Operation::Rollback
        | Operation::Destroy => (
            vec![unimplemented_action_command(action, collection, target)],
            vec!["no domain-specific command plan is generated for this action yet".to_string()],
            true,
        ),
    }
}

fn zfs_pool_command_target<'a>(action: &'a PlannedAction, fallback: Option<&'a str>) -> &'a str {
    action
        .context
        .name
        .as_deref()
        .or(fallback)
        .unwrap_or("<zfs-pool>")
}
