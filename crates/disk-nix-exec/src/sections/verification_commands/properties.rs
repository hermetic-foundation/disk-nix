fn property_verification(
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
        _ => return None,
    })
}
