fn topology_verification(
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
        _ => return None,
    })
}
