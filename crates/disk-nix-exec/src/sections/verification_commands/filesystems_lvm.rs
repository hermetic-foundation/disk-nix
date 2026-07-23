fn filesystem_lvm_verification(
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
        _ => return None,
    })
}
