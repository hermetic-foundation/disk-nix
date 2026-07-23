fn domain_roll_forward_inspection_commands(step: &ExecutionStep) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    if let Some(target) = command_step_target(step) {
        commands.push(command_vec(
            ["disk-nix", "inspect", target, "--json"],
            false,
            "inspect the failed target before choosing roll-forward",
        ));
    }

    match (
        step.operation,
        command_step_collection(step),
        command_step_target(step),
    ) {
        (Operation::Rollback, Some("snapshots"), Some(target)) if is_zfs_snapshot_name(target) => {
            if let Some(dataset) = target.split_once('@').map(|(dataset, _)| dataset) {
                commands.push(command_vec(
                    ["zfs", "list", "-H", "-p", dataset],
                    false,
                    "inspect the dataset that would be rolled forward or retried",
                ));
                commands.push(command_vec(
                    [
                        "zfs",
                        "list",
                        "-t",
                        "snapshot",
                        "-H",
                        "-p",
                        "-o",
                        "name,creation,used,referenced,userrefs",
                        "-r",
                        dataset,
                    ],
                    false,
                    "inspect newer snapshots before completing rollback or choosing roll-forward",
                ));
            }
        }
        (Operation::Rollback, Some("lvmSnapshots"), Some(target)) => {
            commands.push(command_vec(
                ["lvs", "--reportformat", "json", "-a", target],
                false,
                "inspect LVM origin, snapshot, and merge state before roll-forward",
            ));
        }
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("partitions"), _)
        | (Operation::Create | Operation::Rescan, Some("disks"), _) => {
            commands.extend(partition_recovery_inspection_commands(
                step,
                "inspect partition table state before choosing roll-forward",
            ))
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Export
            | Operation::Rescan
            | Operation::SetProperty
            | Operation::Unexport,
            Some("exports"),
            _,
        )
        | (
            Operation::Create
            | Operation::Destroy
            | Operation::Mount
            | Operation::Remount
            | Operation::Rescan
            | Operation::Unmount,
            Some("nfs.mounts"),
            _,
        ) => commands.extend(nfs_recovery_inspection_commands(
            step,
            "inspect NFS state before choosing roll-forward",
        )),
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("backingFiles"), _)
        | (
            Operation::Create | Operation::Destroy | Operation::Grow | Operation::Rescan,
            Some("loopDevices"),
            _,
        )
        | (Operation::Destroy | Operation::Rename | Operation::Rescan, Some("dmMaps"), _) => {
            commands.extend(local_mapping_recovery_inspection_commands(
                step,
                "inspect local mapping state before choosing roll-forward",
            ))
        }
        (
            Operation::AddDevice
            | Operation::Check
            | Operation::Format
            | Operation::Grow
            | Operation::Mount
            | Operation::Rebalance
            | Operation::RemoveDevice
            | Operation::Remount
            | Operation::Repair
            | Operation::Rescan
            | Operation::Scrub
            | Operation::SetProperty
            | Operation::Shrink
            | Operation::Trim
            | Operation::Unmount,
            Some("filesystems"),
            _,
        ) => commands.extend(filesystem_recovery_inspection_commands(
            step,
            "inspect filesystem state before choosing roll-forward",
        )),
        (
            Operation::AddDevice
            | Operation::Create
            | Operation::Destroy
            | Operation::Export
            | Operation::Grow
            | Operation::Import
            | Operation::Promote
            | Operation::Rebalance
            | Operation::RemoveDevice
            | Operation::Rename
            | Operation::ReplaceDevice
            | Operation::Rescan
            | Operation::Scrub
            | Operation::SetProperty,
            Some("pools" | "datasets" | "zvols"),
            _,
        ) => commands.extend(zfs_recovery_inspection_commands(
            step,
            "inspect ZFS state before choosing roll-forward",
        )),
        (
            Operation::Clone
            | Operation::Create
            | Operation::Destroy
            | Operation::Rename
            | Operation::Rescan
            | Operation::SetProperty,
            Some("snapshots"),
            _,
        ) => commands.extend(snapshot_recovery_inspection_commands(
            step,
            "inspect snapshot state before choosing roll-forward",
        )),
        (
            Operation::RemoveDevice | Operation::ReplaceDevice,
            Some("volumeGroups"),
            Some(target),
        ) => {
            commands.push(command_vec(
                ["vgs", "--reportformat", "json", target],
                false,
                "inspect VG allocation and free space before completing device migration",
            ));
            commands.push(command_vec(
                ["pvs", "--reportformat", "json"],
                false,
                "inspect PV allocation before retrying pvmove, vgreduce, or replacement",
            ));
        }
        (
            Operation::AddDevice
            | Operation::Create
            | Operation::Destroy
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rescan
            | Operation::SetProperty,
            Some("caches" | "lvmCaches"),
            _,
        ) => commands.extend(cache_recovery_inspection_commands(
            step,
            "inspect cache state before choosing roll-forward",
        )),
        (
            Operation::Create
            | Operation::Deactivate
            | Operation::Destroy
            | Operation::Format
            | Operation::Grow
            | Operation::Rescan
            | Operation::SetProperty,
            Some("swaps"),
            _,
        ) => commands.extend(swap_recovery_inspection_commands(
            step,
            "inspect swap state before choosing roll-forward",
        )),
        (
            Operation::Attach
            | Operation::Create
            | Operation::Destroy
            | Operation::Detach
            | Operation::Grow
            | Operation::Rescan
            | Operation::SetProperty,
            Some("targetLuns"),
            Some(target),
        ) => commands.extend(target_lun_recovery_inspection_commands(
            Some(target),
            "inspect target-side LUN provider and host-visible path state before choosing roll-forward",
        )),
        (Operation::Destroy | Operation::RemoveDevice | Operation::Detach, Some("luns"), _) => {
            commands.push(command_vec(
                ["disk-nix", "luns", "--json"],
                false,
                "inspect host-side LUN paths before completing detach or cleanup",
            ));
            commands.push(command_vec(
                ["multipath", "-ll"],
                false,
                "inspect multipath maps before retrying LUN path changes",
            ));
        }
        (
            Operation::Create | Operation::Destroy | Operation::Login | Operation::Logout,
            Some("iscsiSessions"),
            Some(target),
        ) => {
            commands.push(command_vec(
                ["iscsiadm", "--mode", "session"],
                false,
                "inspect active iSCSI sessions before choosing roll-forward",
            ));
            commands.push(command_vec(
                ["iscsiadm", "--mode", "node", "--targetname", target],
                false,
                "inspect iSCSI node records before retrying login or logout",
            ));
            commands.push(lsscsi_lun_inventory_command(
                "inspect host-visible LUN transport and size before retrying session changes",
            ));
            commands.push(command_vec(
                ["multipath", "-ll"],
                false,
                "inspect multipath maps before retrying iSCSI session changes",
            ));
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::Attach
            | Operation::Detach,
            Some("nvmeNamespaces"),
            Some(target),
        ) => {
            commands.push(nvme_list_namespaces_command(
                Some(target),
                "inspect NVMe namespace inventory before completing namespace changes",
            ));
            commands.push(nvme_list_subsystems_command(
                "inspect NVMe subsystem and controller attachments before retrying namespace changes",
            ));
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::SetProperty
            | Operation::Start
            | Operation::Stop,
            Some("vdoVolumes"),
            Some(target),
        ) => {
            commands.push(command_vec(
                ["vdo", "status", "--name", target],
                false,
                "inspect VDO volume status before choosing roll-forward",
            ));
            commands.push(command_vec(
                ["vdostats", "--human-readable", target],
                false,
                "inspect VDO utilization and savings counters before retrying",
            ));
            commands.push(command_vec(
                ["disk-nix", "vdo", "--json"],
                false,
                "inspect modeled VDO inventory before retrying lifecycle changes",
            ));
        }
        (
            Operation::AddDevice
            | Operation::Destroy
            | Operation::Grow
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rescan,
            Some("multipathMaps"),
            Some(target),
        ) => {
            commands.push(command_vec(
                ["multipath", "-ll", target],
                false,
                "inspect multipath map paths, policy, and size before choosing roll-forward",
            ));
            commands.push(lsscsi_lun_inventory_command(
                "inspect host-visible SCSI path transport and size before retrying multipath changes",
            ));
            commands.push(command_vec(
                ["disk-nix", "multipath", "--json"],
                false,
                "inspect modeled multipath inventory before retrying lifecycle changes",
            ));
        }
        (
            Operation::Close
            | Operation::Create
            | Operation::Destroy
            | Operation::Format
            | Operation::Grow
            | Operation::Open
            | Operation::SetProperty,
            Some("luks.devices"),
            _,
        )
        | (
            Operation::AddKey
            | Operation::Create
            | Operation::Destroy
            | Operation::RemoveKey
            | Operation::SetProperty,
            Some("luksKeyslots"),
            _,
        )
        | (
            Operation::Create
            | Operation::Destroy
            | Operation::ImportToken
            | Operation::RemoveToken
            | Operation::SetProperty,
            Some("luksTokens"),
            _,
        ) => commands.extend(luks_recovery_inspection_commands(
            step,
            "inspect LUKS state before choosing roll-forward",
        )),
        (
            Operation::Activate
            | Operation::AddDevice
            | Operation::Create
            | Operation::Deactivate
            | Operation::Destroy
            | Operation::Export
            | Operation::Grow
            | Operation::Import
            | Operation::RemoveDevice
            | Operation::Rename
            | Operation::ReplaceDevice
            | Operation::Rescan
            | Operation::Shrink,
            Some("volumes" | "thinPools" | "physicalVolumes" | "volumeGroups"),
            _,
        ) => commands.extend(lvm_recovery_inspection_commands(
            step,
            "inspect LVM state before choosing roll-forward",
        )),
        (
            Operation::Stop
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::AddDevice,
            Some("mdRaids"),
            Some(target),
        ) => {
            commands.push(command_vec(
                ["mdadm", "--detail", target],
                false,
                "inspect MD RAID array health before completing recovery",
            ));
            commands.push(command_vec(
                ["cat", "/proc/mdstat"],
                false,
                "inspect MD RAID sync, recovery, or reshape progress before retrying",
            ));
        }
        _ => {}
    }

    commands
}
