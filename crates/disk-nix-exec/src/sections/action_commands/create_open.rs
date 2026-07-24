fn create_open_action_commands(
    action: &PlannedAction,
    collection: Option<&str>,
    target: Option<&str>,
    cache_target: Option<&str>,
) -> Option<ActionCommandResult> {
    let _ = cache_target;
    Some(match action.operation {
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
                return Some((
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
                ));
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
                partition_udev_settle_command(),
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
        _ => return None,
    })
}
