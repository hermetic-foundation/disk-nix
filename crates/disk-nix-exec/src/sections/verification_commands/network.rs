fn network_verification(
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
        _ => return None,
    })
}
