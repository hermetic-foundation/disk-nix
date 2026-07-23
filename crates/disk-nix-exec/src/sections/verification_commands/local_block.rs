fn local_block_verification(
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
        _ => return None,
    })
}
