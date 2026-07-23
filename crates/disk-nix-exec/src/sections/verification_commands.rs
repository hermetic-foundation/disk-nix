fn verification_for_action(action: &PlannedAction) -> (Vec<ExecutionCommand>, Vec<String>) {
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
        .or_else(|| parts.get(1).copied())
        .unwrap_or("<target>");
    let cache_target = bcache_target_path(action).unwrap_or(target);
    let mountpoint = action.context.mountpoint.as_deref();
    let fs_type = action.context.fs_type.as_deref();
    let desired_size = action.context.desired_size.as_deref();

    match action.operation {
        Operation::Create
        | Operation::Grow
        | Operation::Attach
        | Operation::Detach
        | Operation::Destroy
        | Operation::SetProperty
        | Operation::Rescan
            if collection == Some("targetLuns") || action.id.starts_with("targetLuns:") =>
        {
            (
                target_lun_verification_commands(action, target),
                vec![
                    "target-side provider inventory shows the reviewed LUN identity, initiator mapping, and capacity"
                        .to_string(),
                    "host-side LUN and multipath consumers are refreshed only after provider verification"
                        .to_string(),
                ],
            )
        }
        Operation::Grow
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let mut commands = vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify the post-apply filesystem graph node",
            )];
            if let Some(mountpoint) = mountpoint {
                commands.push(command(
                    ["findmnt", "--json", "--bytes", mountpoint],
                    false,
                    "confirm the mounted filesystem reports the expected capacity",
                ));
            }
            match fs_type {
                Some("btrfs") => commands.push(command(
                    ["btrfs", "filesystem", "usage", "-b", target],
                    false,
                    "inspect Btrfs allocation and free space after resize",
                )),
                Some("zfs") => commands.push(command(
                    ["zfs", "list", "-H", "-p", target],
                    false,
                    "inspect ZFS dataset or zvol size after resize",
                )),
                _ => {}
            }
            (
                commands,
                vec![
                    desired_size
                        .map(|size| format!("filesystem size is at least {size}"))
                        .unwrap_or_else(|| {
                            "filesystem size is at least the desired size".to_string()
                        }),
                    "mountpoint remains present and writable when it was mounted before apply"
                        .to_string(),
                    "free and used byte counters are internally consistent after re-probe"
                        .to_string(),
                ],
            )
        }
        Operation::Shrink
            if collection == Some("filesystems") || action.id.starts_with("filesystem:") =>
        {
            let mut commands = vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify filesystem graph state after the reviewed shrink",
            )];
            if fs_type == Some("btrfs") {
                commands.push(command(
                    ["btrfs", "filesystem", "usage", "-b", target],
                    false,
                    "verify Btrfs allocation and free space after shrink",
                ));
            }
            (
                commands,
                vec![
                    desired_size
                        .map(|size| format!("filesystem size reports no more than {size}"))
                        .unwrap_or_else(|| "filesystem size matches the reviewed shrink target".to_string()),
                    "used data, metadata, and free-space counters remain internally consistent after re-probe"
                        .to_string(),
                    "mounts and dependent services are restored only after filesystem checks pass"
                        .to_string(),
                ],
            )
        }
        Operation::Grow if collection == Some("volumes") || action.id.starts_with("volumes:") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify LVM logical volume size and attributes",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify dependent filesystem and mapping graph state",
                ),
            ],
            vec![
                desired_size
                    .map(|size| format!("logical volume reports size {size}"))
                    .unwrap_or_else(|| "logical volume reports the desired size".to_string()),
                "dependent filesystem capacity reflects the grown backing volume".to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("volumes") || action.id.starts_with("volumes:") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify LVM logical volume attributes after status refresh",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled LV graph relationships after status refresh",
                ),
            ],
            vec![
                "logical volume size, attributes, and activation state are reviewed".to_string(),
                "dependent filesystems, mappings, or mounts still resolve the LV".to_string(),
            ],
        ),
        Operation::Create if collection == Some("volumes") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify LVM logical volume exists after creation",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled LV graph relationships after creation",
                ),
            ],
            vec![
                "logical volume path exists by stable mapper or /dev/<vg>/<lv> name".to_string(),
                "LV size and VG free space match the desired allocation".to_string(),
            ],
        ),
        Operation::Activate | Operation::Deactivate
            if collection == Some("volumes")
                || collection == Some("thinPools")
                || collection == Some("lvmSnapshots") =>
        {
            (
                vec![
                    command(
                        ["lvs", "--reportformat", "json", target],
                        false,
                        "verify LVM logical volume activation state",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify modeled LV graph relationships after activation change",
                    ),
                ],
                vec![
                    "logical volume activation state matches the declared lifecycle operation"
                        .to_string(),
                    "dependent filesystems, mappings, mounts, and services are reviewed after activation state change"
                        .to_string(),
                ],
            )
        }
        Operation::Rescan if collection == Some("lvmSnapshots") => (
            vec![
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify LVM snapshot origin, attributes, and COW usage after rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled LVM snapshot graph relationships after rescan",
                ),
            ],
            vec![
                "snapshot origin, activation state, and COW usage match the refreshed topology"
                    .to_string(),
                "dependent filesystems or recovery mounts still resolve after snapshot status refresh"
                    .to_string(),
            ],
        ),
        Operation::Create if collection == Some("volumeGroups") => (
            vec![
                command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    "verify LVM volume group exists after creation",
                ),
                command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "verify physical volume membership after volume group creation",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled VG graph relationships after creation",
                ),
            ],
            vec![
                "volume group appears with the expected physical volume members".to_string(),
                "VG free extents and metadata state are reviewed before creating LVs".to_string(),
            ],
        ),
        Operation::Grow if collection == Some("volumeGroups") => (
            vec![
                command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    "verify LVM volume group size and free extents after extension",
                ),
                command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "verify physical volume membership after volume group growth",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled VG graph relationships after growth",
                ),
            ],
            vec![
                "volume group includes the expected new physical volume members".to_string(),
                "VG free extents reflect the added capacity before downstream LV growth"
                    .to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("physicalVolumes") => (
            vec![
                command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "verify LVM physical volume inventory after metadata rescan",
                ),
                command(
                    ["vgs", "--reportformat", "json"],
                    false,
                    "verify volume group metadata after PV cache refresh",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after LVM physical volume rescan",
                ),
            ],
            vec![
                "PV metadata, size, and VG membership reflect current block-device state"
                    .to_string(),
                "dependent VGs no longer report stale or missing physical volumes".to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("volumeGroups") => (
            vec![
                command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    "verify LVM volume group metadata after rescan",
                ),
                command(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "verify physical volume membership after VG metadata refresh",
                ),
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify contained logical volumes after VG metadata refresh",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled VG graph relationships after metadata rescan",
                ),
            ],
            vec![
                "volume group metadata and free extents match refreshed PV state".to_string(),
                "logical volumes remain active only where expected after refresh".to_string(),
            ],
        ),
        Operation::Activate | Operation::Deactivate if collection == Some("volumeGroups") => (
            vec![
                command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    "verify LVM volume group activation state",
                ),
                command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    "verify contained logical volume activation state",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled VG graph relationships after activation change",
                ),
            ],
            vec![
                "volume group activation state matches the declared lifecycle operation"
                    .to_string(),
                "contained logical volumes and dependent consumers are reviewed after activation state change"
                    .to_string(),
            ],
        ),
        Operation::Create if collection == Some("datasets") => (
            vec![
                command(
                    ["zfs", "list", "-H", "-p", "-t", "filesystem", target],
                    false,
                    "verify ZFS dataset exists after creation",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled dataset graph relationships after creation",
                ),
            ],
            vec![
                "dataset appears with expected inherited and explicit properties".to_string(),
                "mountpoint, quota, reservation, and encryption policy are reviewed".to_string(),
            ],
        ),
        Operation::Grow if collection == Some("thinPools") => (
            vec![
                command(
                    [
                        "lvs",
                        "--reportformat",
                        "json",
                        "-o",
                        "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                        target,
                    ],
                    false,
                    "verify thin pool size, data usage, metadata usage, and monitoring state",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify thin pool graph node and dependent thin volumes",
                ),
            ],
            vec![
                desired_size
                    .map(|size| format!("thin pool reports size {size}"))
                    .unwrap_or_else(|| "thin pool reports the desired size".to_string()),
                "data and metadata percentages remain below operational thresholds".to_string(),
                "dependent thin volumes remain active and monitored".to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("thinPools") => (
            vec![
                command(
                    [
                        "lvs",
                        "--reportformat",
                        "json",
                        "-o",
                        "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                        target,
                    ],
                    false,
                    "verify thin pool data, metadata, and monitoring state after refresh",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify thin pool graph node and dependent thin volumes after refresh",
                ),
            ],
            vec![
                "thin pool data and metadata utilization are reviewed before further allocation"
                    .to_string(),
                "monitoring and autoextend state match the intended safety policy".to_string(),
            ],
        ),
        Operation::Create if collection == Some("thinPools") => (
            vec![
                command(
                    [
                        "lvs",
                        "--reportformat",
                        "json",
                        "-o",
                        "lv_name,lv_size,data_percent,metadata_percent,seg_monitor",
                        target,
                    ],
                    false,
                    "verify thin pool exists after creation",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify thin pool graph node and volume group relationship after creation",
                ),
            ],
            vec![
                "thin pool reports expected size and monitored state".to_string(),
                "data and metadata utilization are reviewed before thin volumes are created"
                    .to_string(),
            ],
        ),
        Operation::Grow if collection == Some("swaps") => (
            vec![
                command(
                    ["swapon", "--show", "--bytes", "--raw"],
                    false,
                    "verify active swap devices after resize workflow",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify swap graph node and backing storage",
                ),
            ],
            vec![
                "swap target reports the intended capacity".to_string(),
                "swap is active only after backing resize and signature recreation are complete"
                    .to_string(),
            ],
        ),
        Operation::Deactivate if collection == Some("swaps") => (
            vec![
                command(
                    ["swapon", "--show", "--bytes", "--raw"],
                    false,
                    "verify active swap inventory after swapoff",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "verify modeled swap node is inactive or absent after swapoff",
                ),
            ],
            vec![
                "target is absent from active swapon output".to_string(),
                "swap signature remains intact unless a separate destroy action was requested"
                    .to_string(),
            ],
        ),
        Operation::Destroy if collection == Some("swaps") => (
            vec![
                command(
                    ["swapon", "--show", "--bytes", "--raw"],
                    false,
                    "verify active swap inventory after signature removal",
                ),
                swap_blkid_command(
                    swap_target_path(action),
                    "verify swap signature is absent or intentionally replaced",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "verify modeled swap node and consumers after swap destruction",
                ),
            ],
            vec![
                "target is absent from active swapon output".to_string(),
                "NixOS swapDevices, resume, and hibernation references no longer point at the destroyed signature"
                    .to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("swaps") => {
            let target = swap_target_path(action);
            (
                vec![
                    command(
                        ["swapon", "--show", "--bytes", "--raw"],
                        false,
                        "verify active swap inventory after refresh",
                    ),
                    swap_blkid_command(target, "verify swap signature label and UUID after refresh"),
                    swap_inspect_json_command(target, "verify swap graph node and backing storage after refresh"),
                ],
                vec![
                    "swap activation state, size, label, and UUID are reviewed".to_string(),
                    "resume, hibernation, and NixOS swapDevices references still match the refreshed identity"
                        .to_string(),
                ],
            )
        }
        Operation::Rescan if collection == Some("zram") => (
            zram_rescan_commands("verify zram compressed swap inventory after refresh"),
            vec![
                "zram devices, algorithms, sizes, memory use, and swap state are reviewed"
                    .to_string(),
                "NixOS zramSwap settings still match the generated compressed swap topology"
                    .to_string(),
            ],
        ),
        Operation::Create if collection == Some("backingFiles") => {
            let target = backing_file_target_path(action);
            (
                vec![
                    backing_file_stat_command(target, "verify backing file metadata after creation"),
                    backing_file_inspect_json_command(
                        target,
                        "verify modeled backing-file relationships after creation",
                    ),
                ],
                vec![
                    "backing file exists at the reviewed path with the requested capacity"
                        .to_string(),
                    "loop devices, swapfiles, and filesystem consumers are created only after the file identity is verified"
                        .to_string(),
                ],
            )
        }
        Operation::Grow if collection == Some("luks.devices") => (
            vec![
                command(
                    ["cryptsetup", "status", target],
                    false,
                    "verify LUKS mapper state after resize",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify LUKS mapping and dependent graph layers",
                ),
            ],
            vec![
                "LUKS mapper sector count reflects the grown backing device".to_string(),
                "dependent LVM, filesystem, and mount layers see the new mapper capacity"
                    .to_string(),
            ],
        ),
        Operation::Grow if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status", "--name", target],
                    false,
                    "verify VDO volume configuration after growth",
                ),
                command(
                    ["vdostats", "--human-readable", target],
                    false,
                    "verify VDO runtime capacity, utilization, and savings after growth",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify VDO graph node and backing relationships",
                ),
            ],
            vec![
                "VDO logical or physical size matches desired state".to_string(),
                "used, available, and space-saving counters are reviewed after growth".to_string(),
                "dependent filesystems or mappings see the intended capacity".to_string(),
            ],
        ),
        Operation::Create if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status", "--name", target],
                    false,
                    "verify VDO volume configuration after creation",
                ),
                command(
                    ["vdostats", "--human-readable", target],
                    false,
                    "verify VDO runtime capacity and savings counters after creation",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify VDO graph node and backing relationships after creation",
                ),
            ],
            vec![
                "VDO device exists with the intended logical size and backing device".to_string(),
                "deduplication, compression, and write policy are reviewed before use".to_string(),
            ],
        ),
        Operation::Start if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status", "--name", target],
                    false,
                    "verify VDO volume configuration after start",
                ),
                command(
                    ["vdostats", "--human-readable", target],
                    false,
                    "verify VDO runtime counters after start",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify VDO graph node and active consumers after start",
                ),
            ],
            vec![
                "VDO volume is started and reports healthy runtime counters".to_string(),
                "dependent filesystems or mappings see the VDO device before use".to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status", "--name", target],
                    false,
                    "verify VDO volume configuration after status refresh",
                ),
                command(
                    ["vdostats", "--human-readable", target],
                    false,
                    "verify VDO runtime counters after status refresh",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify VDO graph node and backing relationships after status refresh",
                ),
            ],
            vec![
                "VDO volume status and operating mode match expected state".to_string(),
                "utilization, available space, and space-saving counters are reviewed".to_string(),
                "dependent filesystems or mappings still resolve the VDO device".to_string(),
            ],
        ),
        Operation::Grow if collection == Some("zvols") => (
            vec![
                command(
                    ["zfs", "list", "-H", "-p", "-t", "volume", target],
                    false,
                    "verify zvol volsize after growth",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify zvol graph node and dependent block consumers",
                ),
            ],
            vec![
                desired_size
                    .map(|size| format!("zvol volsize reports {size}"))
                    .unwrap_or_else(|| "zvol volsize reports the desired capacity".to_string()),
                "dependent LUNs, guests, partitions, or filesystems see the intended capacity"
                    .to_string(),
            ],
        ),
        Operation::Grow if collection == Some("loopDevices") => (
            vec![
                command(
                    ["losetup", "--json", "--list", target],
                    false,
                    "verify loop device size and backing file after refresh",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify loop graph node and dependent consumers",
                ),
            ],
            vec![
                "loop device reports the refreshed backing size".to_string(),
                "dependent mappings or filesystems see the intended capacity".to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("loopDevices") => (
            vec![
                command(
                    ["losetup", "--json", "--list", target],
                    false,
                    "verify loop device mapping inventory after rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify loop graph node and dependent consumers after rescan",
                ),
            ],
            vec![
                "loop device backing file, offset, sizelimit, and autoclear state are reviewed"
                    .to_string(),
                "dependent mappings or filesystems still resolve the loop device".to_string(),
            ],
        ),
        Operation::Grow if collection == Some("backingFiles") => {
            let target = backing_file_target_path(action);
            (
                vec![
                    backing_file_stat_command(target, "verify backing file size after growth"),
                    backing_file_inspect_json_command(
                        target,
                        "verify modeled backing-file consumers after growth",
                    ),
                ],
                vec![
                    "backing file reports the requested capacity".to_string(),
                    "dependent loop, swap, mapping, or filesystem consumers are refreshed separately"
                        .to_string(),
                ],
            )
        }
        Operation::Rescan if collection == Some("backingFiles") => {
            let target = backing_file_target_path(action);
            (
                vec![
                    backing_file_stat_command(target, "verify backing file metadata after rescan"),
                    backing_file_usage_command(target),
                    backing_file_inspect_json_command(
                        target,
                        "verify modeled backing-file consumers after rescan",
                    ),
                ],
                vec![
                    "backing file size, allocation, and sparse usage are reviewed".to_string(),
                    "dependent loop, swap, mapping, or filesystem consumers still resolve the file"
                        .to_string(),
                ],
            )
        }
        Operation::Rescan if collection == Some("dmMaps") => {
            let target = dm_map_target_path(action);
            (
                vec![
                    dmsetup_info_command(target, "verify device-mapper identity after rescan"),
                    dmsetup_deps_command(target),
                    dmsetup_table_command(target),
                    dmsetup_status_command(target),
                    dm_map_inspect_json_command(
                        target,
                        "verify modeled device-mapper relationships after rescan",
                    ),
                ],
                vec![
                    "device-mapper name, UUID, dependencies, table, and live status are reviewed"
                        .to_string(),
                    "dependent LUKS, LVM, VDO, multipath, filesystem, or mount consumers still resolve the mapper"
                        .to_string(),
                ],
            )
        }
        Operation::Rename if collection == Some("dmMaps") => {
            let rename_to = dm_map_rename_to(action);
            let renamed_target = rename_to
                .as_ref()
                .map(|name| format!("/dev/mapper/{name}"));
            let renamed_target = renamed_target.as_deref();
            (
                vec![
                    dmsetup_info_command(renamed_target, "verify device-mapper identity after rename"),
                    dmsetup_deps_command(renamed_target),
                    dmsetup_status_command(renamed_target),
                    dm_map_inspect_json_command(
                        renamed_target,
                        "verify modeled device-mapper relationships after rename",
                    ),
                ],
                vec![
                    "renamed device-mapper path resolves with the expected name, UUID, dependencies, and status"
                        .to_string(),
                    "dependent LUKS, LVM, VDO, multipath, filesystem, or mount consumers are updated to the new mapper path"
                        .to_string(),
                ],
            )
        }
        Operation::Destroy if collection == Some("dmMaps") => (
            vec![dmsetup_ls_tree_command(
                "verify device-mapper inventory after removal",
            )],
            vec![
                "removed device-mapper map no longer appears in dmsetup inventory".to_string(),
                "dependent mounts, LUKS, LVM, VDO, multipath, cache, or filesystem consumers were removed or moved first"
                    .to_string(),
            ],
        ),
        Operation::Create | Operation::Grow if collection == Some("partitions") => (
            vec![
                command(
                    ["lsblk", "--json", "--bytes", "--output-all"],
                    false,
                    "verify kernel partition and consumer topology",
                ),
                command(
                    ["parted", "-lm"],
                    false,
                    "verify partition table geometry after the change",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify partition graph node and dependent mappings",
                ),
            ],
            vec![
                "partition start, end, size, type, and flags match desired state".to_string(),
                "kernel reread succeeded or a reboot is scheduled before resizing consumers"
                    .to_string(),
                "dependent LUKS, LVM, filesystem, and mount layers still resolve correctly"
                    .to_string(),
            ],
        ),
        Operation::Create if collection == Some("disks") => (
            vec![
                command(
                    ["parted", "-lm", target],
                    false,
                    "verify disk partition table label after initialization",
                ),
                command(
                    ["lsblk", "--json", "--bytes", "--output-all", target],
                    false,
                    "verify kernel disk and partition-table state after reread",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled disk graph node after initialization",
                ),
            ],
            vec![
                "disk reports the requested partition table label".to_string(),
                "no unexpected partitions or consumers remain after initialization".to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("disks") || collection == Some("partitions") => {
            let disk = partition_rescan_disk(action);
            (
                vec![
                    disk_parted_machine_list_command(
                        disk,
                        "verify partition table after kernel reread",
                    ),
                    command(
                        ["disk-nix", "topology", "--json"],
                        false,
                        "verify disk and partition graph after partition-table rescan",
                    ),
                ],
                vec![
                    "kernel partition inventory matches the reviewed table".to_string(),
                    "dependent filesystems, mappings, and mounts still resolve stable paths"
                        .to_string(),
                ],
            )
        }
        Operation::Grow | Operation::Rescan
            if collection == Some("luns")
                || collection == Some("iscsiSessions")
                || action.id.starts_with("luns:")
                || action.id.starts_with("iscsiSessions:") =>
        {
            let is_rescan = action.operation == Operation::Rescan;
            let mut commands = vec![
                command(
                    ["lsblk", "--json", "--bytes", "--output-all"],
                    false,
                    if is_rescan {
                        "verify kernel block-device inventory after host rescan"
                    } else {
                        "verify kernel block-device capacity after host rescan"
                    },
                ),
                lsscsi_lun_inventory_command(if is_rescan {
                    "verify host-visible LUN transport and size after rescan"
                } else {
                    "verify host-visible LUN transport and size after growth rescan"
                }),
            ];
            for device in lun_rescan_devices(action) {
                commands.push(command_vec(
                    vec!["blockdev", "--getsize64", device.as_str()],
                    false,
                    if is_rescan {
                        "verify the reviewed LUN path is visible after rescan"
                    } else {
                        "verify the reviewed LUN path reports its post-rescan byte size"
                    },
                ));
            }
            commands.push(command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify LUN, iSCSI session, multipath, and consumers in the graph",
            ));
            (
                commands,
                vec![
                    if is_rescan {
                        "every expected path remains visible after rescan".to_string()
                    } else {
                        desired_size
                            .map(|size| format!("every expected path reports capacity {size}"))
                            .unwrap_or_else(|| {
                                "every expected path reports the new capacity".to_string()
                            })
                    },
                    if is_rescan {
                        "multipath maps and dependent volumes no longer report stale paths"
                            .to_string()
                    } else {
                        "multipath maps and dependent volumes no longer report stale sizes"
                            .to_string()
                    },
                    "no consumer remains on a missing or failed path".to_string(),
                ],
            )
        }
        Operation::Create | Operation::Attach
            if collection == Some("luns") || action.id.starts_with("luns:") =>
        {
            let mut commands = vec![
                command(
                    ["lsblk", "--json", "--bytes", "--output-all"],
                    false,
                    "verify kernel block-device inventory after LUN attach",
                ),
                lsscsi_lun_inventory_command(
                    "verify attached LUN transport and size after host rescan",
                ),
            ];
            for device in lun_rescan_devices(action) {
                commands.push(command_vec(
                    vec!["blockdev", "--getsize64", device.as_str()],
                    false,
                    "verify the reviewed LUN path exists and reports capacity",
                ));
            }
            commands.push(command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify attached LUN, iSCSI session, multipath, and consumers in the graph",
            ));
            (
                commands,
                vec![
                    "every expected LUN path is visible by a stable device name".to_string(),
                    "multipath maps and consumers are created only after path identity is verified"
                        .to_string(),
                ],
            )
        }
        Operation::Destroy | Operation::Detach
            if collection == Some("luns") || action.id.starts_with("luns:") =>
        {
            let mut commands = vec![
                command(
                    ["lsblk", "--json", "--bytes", "--output-all"],
                    false,
                    "verify kernel block-device inventory after LUN detach",
                ),
                lsscsi_lun_inventory_command(
                    "verify remaining host-visible LUN transport and size after detach",
                ),
            ];
            for device in lun_rescan_devices(action) {
                commands.push(command_vec(
                    vec!["test", "!", "-e", device.as_str()],
                    false,
                    "verify the reviewed LUN path is no longer present",
                ));
            }
            commands.push(command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify detached LUN paths and remaining consumers in the graph",
            ));
            (
                commands,
                vec![
                    "detached LUN paths no longer appear in kernel block inventory".to_string(),
                    "remaining multipath maps, filesystems, and services have no stale dependencies"
                        .to_string(),
                ],
            )
        }
        Operation::Create | Operation::Destroy | Operation::Login | Operation::Logout
            if collection == Some("iscsiSessions") =>
        (
            vec![
                command(
                    ["iscsiadm", "--mode", "session"],
                    false,
                    "list active iSCSI sessions after login or logout",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify iSCSI session, LUN, multipath, and consumer graph state",
                ),
            ],
            vec![
                "session login state matches the declared lifecycle operation".to_string(),
                "dependent LUN paths and multipath maps are present only when expected".to_string(),
            ],
        ),
        Operation::Create | Operation::Mount if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            (
                vec![
                    nfs_mount_findmnt_command(mountpoint),
                    command(
                        ["disk-nix", "inspect", mountpoint.unwrap_or(target), "--json"],
                        false,
                        "verify NFS mount graph state after mount",
                    ),
                ],
                vec![
                    "findmnt reports the mountpoint with the reviewed NFS source and options"
                        .to_string(),
                    "local services see the expected mounted NFS source".to_string(),
                ],
            )
        }
        Operation::Remount if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            (
                vec![
                    nfs_mount_findmnt_command(mountpoint),
                    command(
                        ["disk-nix", "inspect", mountpoint.unwrap_or(target), "--json"],
                        false,
                        "verify NFS mount graph state after remount",
                    ),
                ],
                vec![
                    "findmnt reports the mountpoint with the reviewed NFS options".to_string(),
                    "local services continue to see the expected mount source and filesystem type"
                        .to_string(),
                ],
            )
        }
        Operation::Rescan if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            let inspect_target = mountpoint.unwrap_or("<mountpoint>");
            let inspect_command = match mountpoint {
                Some(mountpoint) => command(
                    ["disk-nix", "inspect", mountpoint, "--json"],
                    false,
                    "verify modeled NFS mount graph state after rescan",
                ),
                None => command_with_readiness(
                    ["disk-nix", "inspect", inspect_target, "--json"],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["mountpoint path"],
                    "verify modeled NFS mount graph state after selecting the mountpoint",
                ),
            };
            (
                vec![
                    nfs_mount_findmnt_command(mountpoint),
                    nfs_mount_stats_command(mountpoint),
                    inspect_command,
                ],
                vec![
                    "findmnt reports the reviewed NFS source and mount options".to_string(),
                    "NFS client statistics are reviewed before remount or unmount work"
                        .to_string(),
                ],
            )
        }
        Operation::Destroy | Operation::Unmount if collection == Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_path(action);
            (
                vec![
                    command(
                        ["findmnt", "--json", mountpoint.unwrap_or("<mountpoint>")],
                        false,
                        "verify NFS mountpoint is no longer mounted",
                    ),
                    command(
                        ["disk-nix", "topology", "--json"],
                        false,
                        "re-probe topology after NFS client unmount",
                    ),
                ],
                vec![
                    "findmnt no longer reports the NFS mountpoint as mounted".to_string(),
                    "local filesystems and services no longer depend on the unmounted path"
                        .to_string(),
                ],
            )
        }
        Operation::AddDevice | Operation::ReplaceDevice | Operation::Rebalance
            if collection == Some("pools") =>
        {
            let target = zfs_pool_command_target(action, Some(target));
            (
                vec![
                    command(
                        ["zpool", "status", "-P", target],
                        false,
                        "verify ZFS pool health and device topology",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify pool graph relationships after topology change",
                    ),
                ],
                vec![
                    "pool is online or degraded only in an explicitly accepted state".to_string(),
                    "new, replaced, or rebalanced devices match desired topology".to_string(),
                    "scrub, resilver, or rebalance status is reviewed to completion".to_string(),
                ],
            )
        }
        Operation::Create if collection == Some("pools") => (
            vec![
                command(
                    [
                        "zpool",
                        "status",
                        "-P",
                        action.context.name.as_deref().unwrap_or(target),
                    ],
                    false,
                    "verify ZFS pool health and vdev topology after creation",
                ),
                command(
                    [
                        "zpool",
                        "list",
                        "-H",
                        "-p",
                        action.context.name.as_deref().unwrap_or(target),
                    ],
                    false,
                    "verify ZFS pool size, allocation, and free capacity after creation",
                ),
                command(
                    [
                        "disk-nix",
                        "inspect",
                        action.context.name.as_deref().unwrap_or(target),
                        "--json",
                    ],
                    false,
                    "verify modeled pool graph relationships after creation",
                ),
            ],
            vec![
                "pool exists with expected vdev devices and health state".to_string(),
                "pool free space and allocation policy are reviewed before creating datasets"
                    .to_string(),
            ],
        ),
        Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::RemoveDevice
        | Operation::Rebalance
            if collection == Some("filesystems") =>
        {
            if action.context.fs_type.as_deref() == Some("bcachefs") {
                (
                    vec![
                        bcachefs_usage_command(
                            target,
                            "verify bcachefs allocation after topology change",
                        ),
                        command(
                            ["disk-nix", "inspect", target, "--json"],
                            false,
                            "verify filesystem graph relationships after topology change",
                        ),
                    ],
                    vec![
                        "bcachefs member list matches desired topology".to_string(),
                        "replication and free-space state are reviewed after topology change"
                            .to_string(),
                    ],
                )
            } else {
                (
                    vec![
                        command(
                            ["btrfs", "filesystem", "usage", "-b", target],
                            false,
                            "verify Btrfs device allocation after topology change",
                        ),
                        command(
                            ["disk-nix", "inspect", target, "--json"],
                            false,
                            "verify filesystem graph relationships after topology change",
                        ),
                    ],
                    vec![
                        "Btrfs device list matches desired topology".to_string(),
                        "allocation profiles remain intentional after rebalance".to_string(),
                    ],
                )
            }
        }
        Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::RemoveDevice
        | Operation::Create
        | Operation::Assemble
        | Operation::Stop
        | Operation::Grow
            if collection == Some("mdRaids") =>
        {
            (
                vec![
                    command(
                        ["mdadm", "--detail", target],
                        false,
                        "verify MD RAID array health and member topology",
                    ),
                    command(
                        ["cat", "/proc/mdstat"],
                        false,
                        "verify MD RAID sync, recovery, or reshape state",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify MD RAID graph relationships after topology change",
                    ),
                ],
                vec![
                    "array is clean or intentionally syncing, recovering, or reshaping".to_string(),
                    "member list and redundancy match the desired topology".to_string(),
                    "dependent filesystems or mappings see the expected capacity".to_string(),
                ],
            )
        }
        Operation::Rescan if collection == Some("mdRaids") => {
            let target = md_array_target_path(action);
            let mut commands = Vec::new();
            if let Some(target) = target {
                commands.push(command(
                    ["mdadm", "--detail", target],
                    false,
                    "verify targeted MD RAID array state after metadata rescan",
                ));
            }
            commands.extend([
                command(
                    ["mdadm", "--detail", "--scan"],
                    false,
                    "verify assembled MD RAID array inventory after metadata rescan",
                ),
                command(
                    ["mdadm", "--examine", "--scan"],
                    false,
                    "verify member metadata inventory after MD RAID rescan",
                ),
                command(
                    ["cat", "/proc/mdstat"],
                    false,
                    "verify MD RAID kernel status after metadata rescan",
                ),
                command(
                    ["disk-nix", "topology", "--json"],
                    false,
                    "re-probe topology after MD RAID metadata rescan",
                ),
            ]);
            (
                commands,
                vec![
                    "array metadata inventory matches the reviewed member devices".to_string(),
                    "no unexpected arrays are assembled or missing after metadata refresh"
                        .to_string(),
                    "dependent filesystems and mappings still reference expected MD devices"
                        .to_string(),
                ],
            )
        }
        Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::RemoveDevice
        | Operation::Grow
        | Operation::Rescan
            if collection == Some("multipathMaps") =>
        {
            (
                vec![
                    command(
                        ["multipath", "-ll", target],
                        false,
                        "verify multipath map paths, policy, and size",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify multipath graph relationships after path or map change",
                    ),
                ],
                vec![
                    "all expected paths are active or intentionally failed".to_string(),
                    if action.operation == Operation::Rescan {
                        "map WWID and path state still match the reviewed topology".to_string()
                    } else {
                        "map size and WWID match desired state".to_string()
                    },
                    "dependent filesystems or mappings see the expected capacity".to_string(),
                ],
            )
        }
        Operation::Destroy if collection == Some("multipathMaps") => (
            vec![
                command(
                    ["multipath", "-ll"],
                    false,
                    "verify multipath inventory after map removal",
                ),
                command(
                    ["disk-nix", "inspect", "multipath", "--json"],
                    false,
                    "verify multipath graph relationships after map removal",
                ),
            ],
            vec![
                "removed multipath map no longer appears in host multipath inventory".to_string(),
                "dependent filesystems, LVM, dm, and service consumers were removed or moved first"
                    .to_string(),
            ],
        ),
        Operation::Create
        | Operation::Attach
        | Operation::Grow
        | Operation::Detach
        | Operation::Destroy
            if collection == Some("nvmeNamespaces") =>
        {
            (
                vec![
                    command(
                        ["nvme", "list", "--output-format=json"],
                        false,
                        "verify NVMe controller and namespace inventory",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify NVMe namespace graph relationships after lifecycle change",
                    ),
                ],
                vec![
                    "namespace id, attachment state, and capacity match the desired lifecycle outcome"
                        .to_string(),
                    "dependent partitions, volumes, or filesystems see the expected namespace state"
                        .to_string(),
                ],
            )
        }
        Operation::Create | Operation::Grow | Operation::Destroy
            if collection == Some("physicalVolumes") =>
        {
            (
                vec![
                    command(
                        ["pvs", "--reportformat", "json"],
                        false,
                        "verify LVM physical volume inventory after lifecycle change",
                    ),
                    command(
                        ["disk-nix", "inspect", target, "--json"],
                        false,
                        "verify physical volume graph relationships after lifecycle change",
                    ),
                ],
                vec![
                    "PV metadata, size, and VG membership match the desired state".to_string(),
                    "dependent volume groups report expected free extents".to_string(),
                ],
            )
        }
        Operation::Create
        | Operation::AddKey
        | Operation::SetProperty
        | Operation::Destroy
        | Operation::RemoveKey
            if collection == Some("luksKeyslots") =>
        {
            let device = luks_keyslot_device(action);
            (
                vec![
                    luks_dump_command(device, "verify LUKS header and keyslot metadata"),
                    command(
                        ["disk-nix", "inspect", device.unwrap_or("<luks-device>"), "--json"],
                        false,
                        "verify modeled LUKS container relationships after keyslot change",
                    ),
                ],
                vec![
                    "at least one reviewed keyslot or token remains usable after the change"
                        .to_string(),
                    "LUKS header backup and keyslot inventory match the desired access policy"
                        .to_string(),
                ],
            )
        }
        Operation::Create
        | Operation::ImportToken
        | Operation::SetProperty
        | Operation::Destroy
        | Operation::RemoveToken
            if collection == Some("luksTokens") =>
        {
            let device = luks_token_device(action);
            (
                vec![
                    luks_dump_command(device, "verify LUKS header and token metadata"),
                    command(
                        ["disk-nix", "inspect", device.unwrap_or("<luks-device>"), "--json"],
                        false,
                        "verify modeled LUKS container relationships after token change",
                    ),
                ],
                vec![
                    "at least one reviewed keyslot or token remains usable after the change"
                        .to_string(),
                    "LUKS header backup and token inventory match the desired access policy"
                        .to_string(),
                ],
            )
        }
        Operation::Create
        | Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::RemoveDevice
        | Operation::SetProperty
        | Operation::Destroy
        | Operation::Rescan
            if collection == Some("lvmCaches") =>
        {
            let target = lvm_volume_target_path(Some(target));
            (
                vec![
                    lvm_lvs_report_command(
                        target,
                        Some("lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent"),
                        "verify LVM cache state after lifecycle change",
                    ),
                    command(
                        ["disk-nix", "inspect", target.unwrap_or("<lvm-cache>"), "--json"],
                        false,
                        "verify modeled LVM cache relationships after cache update",
                    ),
                ],
                vec![
                    "origin LV, cache pool, cache mode, and dirty data state match the desired cache lifecycle"
                        .to_string(),
                    "origin LV remains readable after cache attach, detach, or mode update".to_string(),
                ],
            )
        }
        Operation::AddDevice
        | Operation::ReplaceDevice
        | Operation::RemoveDevice
        | Operation::SetProperty
        | Operation::Rescan
            if collection == Some("caches") =>
        {
            (
                vec![
                    command(
                        ["disk-nix", "inspect", cache_target, "--json"],
                        false,
                        "verify modeled cache layer relationships after cache update",
                    ),
                    bcache_sysfs_read_command(
                        cache_target,
                        "state",
                        "verify bcache state after update",
                    ),
                    bcache_sysfs_read_command(
                        cache_target,
                        "dirty_data",
                        "verify dirty data after cache update",
                    ),
                ],
                vec![
                    "cache backing device and cache-set relationships match desired topology"
                        .to_string(),
                    "dirty writeback data is flushed before detach or replacement".to_string(),
                    "cache mode matches the desired safety posture after the operation".to_string(),
                ],
            )
        }
        Operation::AddDevice | Operation::ReplaceDevice | Operation::Rebalance => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify storage topology after device-level operation",
            )],
            vec!["target topology and health match the desired state".to_string()],
        ),
        Operation::SetProperty if collection == Some("pools") => (
            vec![
                command(
                    ["zpool", "get", "all", target],
                    false,
                    "verify ZFS pool properties after update",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled pool properties after re-probe",
                ),
            ],
            vec!["changed property equals the desired value".to_string()],
        ),
        Operation::SetProperty if collection == Some("datasets") => (
            vec![
                command(
                    ["zfs", "get", "all", target],
                    false,
                    "verify ZFS dataset properties after update",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled dataset properties after re-probe",
                ),
            ],
            vec!["changed property equals the desired value".to_string()],
        ),
        Operation::SetProperty if collection == Some("zvols") => (
            vec![
                command(
                    ["zfs", "get", "all", target],
                    false,
                    "verify zvol properties after update",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled zvol properties after re-probe",
                ),
            ],
            vec!["changed property equals the desired value".to_string()],
        ),
        Operation::Rescan if collection == Some("datasets") => (
            vec![
                command(
                    ["zfs", "list", "-H", "-p", "-t", "filesystem", target],
                    false,
                    "verify ZFS dataset inventory after rescan",
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
                    "verify ZFS dataset properties after rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled ZFS dataset graph state after rescan",
                ),
            ],
            vec![
                "dataset properties, mountpoint, and inherited policy match refreshed inventory"
                    .to_string(),
                "snapshot, clone, mount, and export relationships are reviewed".to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("zvols") => (
            vec![
                command(
                    ["zfs", "list", "-H", "-p", "-t", "volume", target],
                    false,
                    "verify zvol inventory after rescan",
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
                    "verify zvol properties after rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled zvol block graph state after rescan",
                ),
            ],
            vec![
                "zvol volsize, reservation, and property state match refreshed inventory"
                    .to_string(),
                "dependent LUN, guest, partition, and filesystem consumers are reviewed"
                    .to_string(),
            ],
        ),
        Operation::SetProperty if collection == Some("vdoVolumes") => (
            vec![
                command(
                    ["vdo", "status", "--name", target],
                    false,
                    "verify VDO configuration after property update",
                ),
                command(
                    ["vdostats", "--verbose", target],
                    false,
                    "verify VDO runtime mode and policy after property update",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled VDO properties after re-probe",
                ),
            ],
            vec!["changed VDO property equals the desired value".to_string()],
        ),
        Operation::SetProperty if collection == Some("luks.devices") => {
            let device = action.context.device.as_deref();
            (
                vec![
                    luks_dump_command(device, "verify LUKS header metadata after property update"),
                    command(
                        ["disk-nix", "inspect", device.unwrap_or("<luks-device>"), "--json"],
                        false,
                        "verify modeled LUKS header properties after re-probe",
                    ),
                ],
                vec![
                    "changed LUKS header property equals the desired value".to_string(),
                    "initrd, crypttab, and dependent mappings still reference the intended encrypted container"
                        .to_string(),
                ],
            )
        }
        Operation::SetProperty if collection == Some("btrfsQgroups") => (
            vec![
                command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "verify Btrfs qgroup limits and usage after update",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled Btrfs qgroup properties after re-probe",
                ),
            ],
            vec!["changed qgroup limit equals the desired value".to_string()],
        ),
        Operation::Create | Operation::Destroy if collection == Some("btrfsQgroups") => (
            vec![
                command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "verify Btrfs qgroup inventory after lifecycle change",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled Btrfs qgroup topology after re-probe",
                ),
            ],
            vec!["Btrfs qgroup hierarchy and limits match desired state".to_string()],
        ),
        Operation::Rescan if collection == Some("btrfsQgroups") => (
            vec![
                command(
                    ["btrfs", "qgroup", "show", "--raw", "-reF", target],
                    false,
                    "verify Btrfs qgroup usage and hierarchy after rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled Btrfs qgroup relationships after re-probe",
                ),
            ],
            vec![
                "Btrfs qgroup referenced and exclusive usage match refreshed topology"
                    .to_string(),
                "qgroup limits and hierarchy are reviewed before later enforcement changes"
                    .to_string(),
            ],
        ),
        Operation::Rescan if collection == Some("btrfsSubvolumes") => (
            vec![
                command(
                    ["btrfs", "subvolume", "show", target],
                    false,
                    "verify Btrfs subvolume metadata after rescan",
                ),
                command(
                    ["btrfs", "property", "get", "-ts", target, "ro"],
                    false,
                    "verify Btrfs subvolume read-only property after rescan",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled Btrfs subvolume relationships after re-probe",
                ),
            ],
            vec![
                "Btrfs subvolume metadata and read-only state match refreshed topology"
                    .to_string(),
                "snapshot and qgroup relationships are reviewed before later cleanup"
                    .to_string(),
            ],
        ),
        Operation::SetProperty if collection == Some("exports") => (
            vec![
                command(
                    ["exportfs", "-v"],
                    false,
                    "verify exported NFS paths and options",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled NFS export properties after re-probe",
                ),
            ],
            vec!["exported path and options match the desired value".to_string()],
        ),
        Operation::Rescan if collection == Some("exports") => {
            let target = export_target_path(action);
            let inspect_target = target.unwrap_or("<export-path>");
            let inspect_command = match target {
                Some(target) => command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled NFS export graph state after rescan",
                ),
                None => command_with_readiness(
                    ["disk-nix", "inspect", inspect_target, "--json"],
                    false,
                    CommandReadiness::NeedsDomainImplementation,
                    ["NFS export path"],
                    "verify modeled NFS export graph state after selecting the export path",
                ),
            };
            (
                vec![
                    command(
                        ["exportfs", "-v"],
                        false,
                        "verify NFS export inventory after rescan",
                    ),
                    inspect_command,
                ],
                vec![
                    "exportfs reports the reviewed path and client options".to_string(),
                    "modeled NFS export relationships match the refreshed inventory".to_string(),
                ],
            )
        }
        Operation::SetProperty if collection == Some("snapshots") => {
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            (
                vec![
                    snapshot_hold_list_command(snapshot),
                    command(
                        ["disk-nix", "inspect", snapshot, "--json"],
                        false,
                        "verify modeled snapshot properties after re-probe",
                    ),
                ],
                vec!["snapshot hold state matches the desired retention tag".to_string()],
            )
        }
        Operation::SetProperty if collection == Some("zram") => (
            zram_rescan_commands("verify zram compressed swap declaration after inventory refresh"),
            vec![
                "zram device count, algorithm, size, memory use, and swap activation are reviewed"
                    .to_string(),
                "NixOS zramSwap-derived settings match the generated compressed swap topology"
                    .to_string(),
            ],
        ),
        Operation::Create | Operation::Export | Operation::Destroy | Operation::Unexport
            if collection == Some("exports") =>
        (
            vec![
                command(
                    ["exportfs", "-v"],
                    false,
                    "verify exported NFS paths and client selectors",
                ),
                command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    "verify modeled NFS export relationships after re-probe",
                ),
            ],
            vec![
                "export path, client selector, and options match desired state".to_string(),
                "remote clients are intentionally added, migrated, or drained".to_string(),
            ],
        ),
        Operation::SetProperty => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify modeled storage properties after re-probe",
            )],
            vec!["changed property equals the desired value".to_string()],
        ),
        Operation::Snapshot => {
            let snapshot = action.context.name.as_deref().unwrap_or(target);
            let mut commands = vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify snapshot target and relationships after creation",
            )];
            if is_zfs_snapshot_name(snapshot) {
                commands.push(command(
                    ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                    false,
                    "verify ZFS snapshot existence and metadata",
                ));
            } else if collection == Some("btrfsSubvolumes")
                || is_btrfs_snapshot_pair(target, snapshot)
            {
                commands.push(command(
                    ["btrfs", "subvolume", "show", snapshot],
                    false,
                    "verify Btrfs snapshot subvolume existence and metadata",
                ));
            } else if collection == Some("lvmSnapshots") {
                commands.push(command(
                    ["lvs", "--reportformat", "json", snapshot],
                    false,
                    "verify LVM snapshot existence and attributes",
                ));
            }
            (
                commands,
                vec![
                    "snapshot exists with the expected name".to_string(),
                    "snapshot source still resolves to the intended dataset or volume".to_string(),
                ],
            )
        }
        Operation::Rescan if collection == Some("snapshots") => {
            let snapshot = snapshot_rescan_identity(action, target);
            let mut commands = vec![command(
                ["disk-nix", "inspect", snapshot, "--json"],
                false,
                "verify modeled snapshot graph relationships after metadata refresh",
            )];
            if is_zfs_snapshot_name(snapshot) {
                commands.push(zfs_snapshot_list_command(
                    snapshot,
                    "verify ZFS snapshot size and reference metadata after rescan",
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
                    "verify ZFS snapshot properties and retention metadata after rescan",
                ));
                commands.push(snapshot_hold_list_command(snapshot));
            } else if snapshot.starts_with('/') {
                commands.push(command(
                    ["btrfs", "subvolume", "show", snapshot],
                    false,
                    "verify Btrfs snapshot subvolume metadata after rescan",
                ));
                commands.push(command(
                    ["btrfs", "property", "get", "-ts", snapshot, "ro"],
                    false,
                    "verify Btrfs snapshot read-only property after rescan",
                ));
            }
            (
                commands,
                vec![
                    "snapshot metadata, source relationship, and retention state match the refreshed topology"
                        .to_string(),
                ],
            )
        }
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
                return (Vec::new(), Vec::new());
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
        Operation::Format
        | Operation::Shrink
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
        | Operation::Repair
        | Operation::Rollback
        | Operation::Destroy => (
            vec![command(
                ["disk-nix", "topology", "--json"],
                false,
                "re-probe full topology after high-risk operation",
            )],
            vec!["operator performs explicit high-risk post-change validation".to_string()],
        ),
        Operation::Grow => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify target state after grow operation",
            )],
            vec!["target capacity and consumers match desired state".to_string()],
        ),
        Operation::Check => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify target state after filesystem check",
            )],
            vec!["read-only check completed and no repair action was applied".to_string()],
        ),
        Operation::Scrub => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify target state after scrub operation",
            )],
            vec!["scrub completed or is running with reviewed health status".to_string()],
        ),
        Operation::Trim => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify target state after trim operation",
            )],
            vec!["filesystem remains mounted and reports consistent usage after trim".to_string()],
        ),
    }
}
