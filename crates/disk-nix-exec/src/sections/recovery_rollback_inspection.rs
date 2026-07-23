fn domain_rollback_inspection_commands(step: &ExecutionStep) -> Vec<ExecutionCommand> {
    match (
        step.operation,
        command_step_collection(step),
        command_step_target(step),
    ) {
        (Operation::Rollback, Some("snapshots"), Some(target)) if is_zfs_snapshot_name(target) => {
            let mut commands = vec![command_vec(
                ["zfs", "list", "-t", "snapshot", "-H", "-p", target],
                false,
                "confirm the rollback point still exists before any retry",
            )];
            if let Some(dataset) = target.split_once('@').map(|(dataset, _)| dataset) {
                commands.push(command_vec(
                    ["zfs", "list", "-H", "-p", dataset],
                    false,
                    "inspect the dataset state that rollback would replace",
                ));
            }
            commands
        }
        (Operation::Rollback, Some("lvmSnapshots"), Some(target)) => vec![command_vec(
            ["lvs", "--reportformat", "json", "-a", target],
            false,
            "confirm the LVM snapshot and origin state before retrying merge rollback",
        )],
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("partitions"), _)
        | (Operation::Create | Operation::Rescan, Some("disks"), _) => {
            partition_recovery_inspection_commands(
                step,
                "confirm partition table state before rollback decisions",
            )
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
        ) => nfs_recovery_inspection_commands(step, "confirm NFS state before rollback decisions"),
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("backingFiles"), _)
        | (
            Operation::Create | Operation::Destroy | Operation::Grow | Operation::Rescan,
            Some("loopDevices"),
            _,
        )
        | (Operation::Destroy | Operation::Rename | Operation::Rescan, Some("dmMaps"), _) => {
            local_mapping_recovery_inspection_commands(
                step,
                "confirm local mapping state before rollback decisions",
            )
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
        ) => filesystem_recovery_inspection_commands(
            step,
            "confirm filesystem state before rollback decisions",
        ),
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
        ) => zfs_recovery_inspection_commands(step, "confirm ZFS state before rollback decisions"),
        (
            Operation::Clone
            | Operation::Create
            | Operation::Destroy
            | Operation::Rename
            | Operation::Rescan
            | Operation::SetProperty,
            Some("snapshots"),
            _,
        ) => snapshot_recovery_inspection_commands(
            step,
            "confirm snapshot state before rollback decisions",
        ),
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Rename
            | Operation::Rescan
            | Operation::SetProperty,
            Some("btrfsSubvolumes"),
            Some(target),
        ) => vec![
            command_vec(
                ["btrfs", "subvolume", "show", target],
                false,
                "confirm Btrfs subvolume state before rollback decisions",
            ),
            command_vec(
                ["btrfs", "property", "get", "-ts", target, "ro"],
                false,
                "confirm Btrfs subvolume read-only state before rollback decisions",
            ),
        ],
        (
            Operation::Create | Operation::Destroy | Operation::Rescan | Operation::SetProperty,
            Some("btrfsQgroups"),
            Some(target),
        ) => vec![command_vec(
            ["btrfs", "qgroup", "show", "--raw", "-reF", target],
            false,
            "confirm Btrfs qgroup limits and usage before rollback decisions",
        )],
        (
            Operation::RemoveDevice | Operation::ReplaceDevice,
            Some("volumeGroups"),
            Some(target),
        ) => {
            vec![
                command_vec(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    "confirm VG metadata before undoing a partially completed PV migration",
                ),
                command_vec(
                    ["pvs", "--reportformat", "json"],
                    false,
                    "confirm whether extents remain on the source or replacement PV",
                ),
            ]
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
        ) => cache_recovery_inspection_commands(
            step,
            "confirm cache state before rollback decisions",
        ),
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
        ) => {
            swap_recovery_inspection_commands(step, "confirm swap state before rollback decisions")
        }
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
        ) => target_lun_recovery_inspection_commands(
            Some(target),
            "confirm target-side LUN provider and host-visible path state before rollback decisions",
        ),
        (
            Operation::Attach
            | Operation::Create
            | Operation::Destroy
            | Operation::Detach
            | Operation::Grow
            | Operation::RemoveDevice
            | Operation::Rescan,
            Some("luns"),
            _,
        ) => vec![
            command_vec(
                ["disk-nix", "luns", "--json"],
                false,
                "confirm host-side LUN paths before rollback decisions",
            ),
            lsscsi_lun_inventory_command(
                "confirm host-visible LUN transport and size before rollback decisions",
            ),
            command_vec(
                ["multipath", "-ll"],
                false,
                "confirm path grouping before restoring or removing multipath maps",
            ),
        ],
        (
            Operation::Create | Operation::Destroy | Operation::Login | Operation::Logout,
            Some("iscsiSessions"),
            Some(target),
        ) => vec![
            command_vec(
                ["iscsiadm", "--mode", "session"],
                false,
                "confirm active iSCSI sessions before rollback decisions",
            ),
            command_vec(
                ["iscsiadm", "--mode", "node", "--targetname", target],
                false,
                "confirm iSCSI node records before undoing or retrying session changes",
            ),
            lsscsi_lun_inventory_command(
                "confirm host-visible LUN paths before rollback decisions",
            ),
            command_vec(
                ["multipath", "-ll"],
                false,
                "confirm multipath path grouping before rollback decisions",
            ),
        ],
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::Attach
            | Operation::Detach,
            Some("nvmeNamespaces"),
            Some(target),
        ) => vec![
            nvme_list_namespaces_command(
                Some(target),
                "confirm NVMe namespace inventory before undoing or retrying namespace changes",
            ),
            nvme_list_subsystems_command(
                "confirm NVMe subsystem attachments before rollback decisions",
            ),
        ],
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::SetProperty
            | Operation::Start
            | Operation::Stop,
            Some("vdoVolumes"),
            Some(target),
        ) => vec![
            command_vec(
                ["vdo", "status", "--name", target],
                false,
                "confirm VDO status before undoing or retrying lifecycle changes",
            ),
            command_vec(
                ["vdostats", "--human-readable", target],
                false,
                "confirm VDO utilization and savings counters before rollback decisions",
            ),
            command_vec(
                ["disk-nix", "vdo", "--json"],
                false,
                "confirm modeled VDO inventory before rollback decisions",
            ),
        ],
        (
            Operation::AddDevice
            | Operation::Destroy
            | Operation::Grow
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rescan,
            Some("multipathMaps"),
            Some(target),
        ) => vec![
            command_vec(
                ["multipath", "-ll", target],
                false,
                "confirm multipath map paths, policy, and size before rollback decisions",
            ),
            lsscsi_lun_inventory_command(
                "confirm host-visible SCSI paths before rollback decisions",
            ),
            command_vec(
                ["disk-nix", "multipath", "--json"],
                false,
                "confirm modeled multipath inventory before rollback decisions",
            ),
        ],
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
        ) => {
            luks_recovery_inspection_commands(step, "confirm LUKS state before rollback decisions")
        }
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
        ) => lvm_recovery_inspection_commands(step, "confirm LVM state before rollback decisions"),
        (
            Operation::Stop
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::AddDevice,
            Some("mdRaids"),
            Some(target),
        ) => vec![
            command_vec(
                ["mdadm", "--detail", target],
                false,
                "confirm MD RAID array health before undoing or retrying member changes",
            ),
            command_vec(
                ["cat", "/proc/mdstat"],
                false,
                "confirm sync, recovery, or reshape state before rollback decisions",
            ),
        ],
        _ => Vec::new(),
    }
}
