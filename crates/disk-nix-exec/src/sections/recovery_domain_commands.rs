fn domain_recovery_commands(step: &ExecutionStep) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    match (
        step.operation,
        command_step_collection(step),
        command_step_target(step),
    ) {
        (Operation::Rollback, Some("snapshots"), Some(target)) if is_zfs_snapshot_name(target) => {
            commands.push(command(
                ["zfs", "list", "-t", "snapshot", "-H", "-p", target],
                false,
                "inspect the rollback snapshot before deciding whether to retry or roll forward",
            ));
            if let Some(dataset) = target.split_once('@').map(|(dataset, _)| dataset) {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", dataset],
                    false,
                    "inspect the rolled-back dataset state after the failed rollback attempt",
                ));
            }
        }
        (Operation::Rollback, Some("lvmSnapshots"), Some(target)) => {
            commands.push(command(
                ["lvs", "--reportformat", "json", target],
                false,
                "inspect LVM snapshot and merge state before deciding whether to retry",
            ));
        }
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("partitions"), _)
        | (Operation::Create | Operation::Rescan, Some("disks"), _) => {
            commands.extend(partition_recovery_inspection_commands(
                step,
                "inspect partition table state after the failed command",
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
            "inspect NFS state after the failed command",
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
                "inspect local mapping state after the failed command",
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
            "inspect filesystem state after the failed command",
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
            "inspect ZFS state after the failed command",
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
            "inspect snapshot state after the failed command",
        )),
        (
            Operation::Create | Operation::Destroy | Operation::Login | Operation::Logout,
            Some("iscsiSessions"),
            Some(target),
        ) => {
            commands.push(command(
                ["iscsiadm", "--mode", "session"],
                false,
                "inspect active iSCSI sessions before deciding whether to retry",
            ));
            commands.push(command(
                ["iscsiadm", "--mode", "node", "--targetname", target],
                false,
                "inspect iSCSI node records after the failed session command",
            ));
            commands.push(lsscsi_lun_inventory_command(
                "inspect host-visible LUN paths after the failed session command",
            ));
            commands.push(command(
                ["multipath", "-ll"],
                false,
                "inspect multipath maps after the failed session command",
            ));
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
        ) => commands.extend(target_lun_recovery_inspection_commands(
            Some(target),
            "inspect target-side LUN provider and host-visible path state after the failed command",
        )),
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
                "inspect NVMe namespace inventory before deciding whether to retry",
            ));
            commands.push(nvme_list_subsystems_command(
                "inspect NVMe subsystem attachments after the failed namespace command",
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
            "inspect cache state after the failed command",
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
            "inspect swap state after the failed command",
        )),
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
            commands.push(command(
                ["vdo", "status", "--name", target],
                false,
                "inspect VDO volume status after the failed lifecycle command",
            ));
            commands.push(command(
                ["vdostats", "--human-readable", target],
                false,
                "inspect VDO utilization and savings counters after the failed lifecycle command",
            ));
            commands.push(command(
                ["disk-nix", "vdo", "--json"],
                false,
                "inspect modeled VDO inventory before deciding whether to retry",
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
            commands.push(command(
                ["multipath", "-ll", target],
                false,
                "inspect multipath map paths, policy, and size after the failed command",
            ));
            commands.push(lsscsi_lun_inventory_command(
                "inspect host-visible SCSI paths after the failed multipath command",
            ));
            commands.push(command(
                ["disk-nix", "multipath", "--json"],
                false,
                "inspect modeled multipath inventory before deciding whether to retry",
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
            "inspect LUKS state after the failed command",
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
            "inspect LVM state after the failed command",
        )),
        (
            Operation::Stop
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::AddDevice,
            Some("mdRaids"),
            Some(target),
        ) => {
            commands.push(command(
                ["mdadm", "--detail", target],
                false,
                "inspect MD RAID member, failed, spare, and recovery state before deciding whether to retry",
            ));
            commands.push(command(
                ["cat", "/proc/mdstat"],
                false,
                "inspect MD RAID runtime recovery or reshape state after the failed command",
            ));
        }
        (_, _, Some(target)) => {
            commands.push(command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "inspect the failed action target before choosing rollback or roll-forward",
            ));
        }
        _ => {}
    }
    commands.extend(state_inspection_commands());
    commands
}

fn domain_recovery_notes(
    step: &ExecutionStep,
    failed: &ExecutionCommandResult,
    completed_mutating_commands: usize,
) -> Vec<String> {
    let mut notes = vec![
        format!(
            "{completed_mutating_commands} mutating command(s) completed before the failed command in this apply run"
        ),
        format!(
            "failed {:?} command for {}: {}",
            failed.phase,
            failed.action_id,
            failed.argv.join(" ")
        ),
        "do not retry the failed action until fresh topology proves whether the target already changed".to_string(),
    ];

    match (step.operation, command_step_collection(step)) {
        (Operation::Rollback, Some("snapshots")) => {
            notes.push(
                "for ZFS rollback, prefer cloning the snapshot or taking a fresh snapshot of the current dataset before any retry".to_string(),
            );
            notes.push(
                "review newer snapshots, clones, mountpoints, shares, and dependent services before choosing rollback or roll-forward".to_string(),
            );
        }
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
        ) => {
            notes.push(
                "for ZFS changes, inspect pool health, dataset or zvol properties, snapshots, clones, mountpoints, shares, and LUN consumers before retrying".to_string(),
            );
            notes.push(
                "prefer read-only import, clone, or fresh snapshot workflows until pool state and dependent services match the intended topology".to_string(),
            );
        }
        (
            Operation::Clone
            | Operation::Create
            | Operation::Destroy
            | Operation::Rename
            | Operation::Rescan
            | Operation::SetProperty,
            Some("snapshots"),
        ) => {
            notes.push(
                "for snapshot lifecycle changes, inspect source, target, hold tags, read-only state, and dependent clones before retrying".to_string(),
            );
            notes.push(
                "prefer preserving or cloning recovery snapshots until retention, rollback, replication, and mount consumers are verified".to_string(),
            );
        }
        (Operation::Rollback, Some("lvmSnapshots")) => {
            notes.push(
                "for LVM snapshot merge rollback, inspect origin activation and merge status before rerunning lvconvert --merge".to_string(),
            );
            notes.push(
                "keep the origin, snapshot, and VG metadata backups intact until the merge outcome is verified".to_string(),
            );
        }
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("partitions"))
        | (Operation::Create | Operation::Rescan, Some("disks")) => {
            notes.push(
                "for partition-table changes, inspect disk identity, partition geometry, kernel reread state, and dependent LUKS, LVM, filesystem, and mount consumers before retrying".to_string(),
            );
            notes.push(
                "preserve partition table captures and avoid formatting or resizing upper layers until the kernel and modeled topology agree on the new geometry".to_string(),
            );
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Export
            | Operation::Rescan
            | Operation::SetProperty
            | Operation::Unexport,
            Some("exports"),
        )
        | (
            Operation::Create
            | Operation::Destroy
            | Operation::Mount
            | Operation::Remount
            | Operation::Rescan
            | Operation::Unmount,
            Some("nfs.mounts"),
        ) => {
            notes.push(
                "for NFS changes, inspect exported paths, client selectors, negotiated mount options, mount state, and dependent services before retrying".to_string(),
            );
            notes.push(
                "keep local services quiesced and preserve declarative export or mount configuration until live NFS state matches the intended topology".to_string(),
            );
        }
        (Operation::Create | Operation::Grow | Operation::Rescan, Some("backingFiles"))
        | (
            Operation::Create | Operation::Destroy | Operation::Grow | Operation::Rescan,
            Some("loopDevices"),
        )
        | (Operation::Destroy | Operation::Rename | Operation::Rescan, Some("dmMaps")) => {
            notes.push(
                "for local mapping changes, inspect backing file size, loop mappings, device-mapper tables, dependencies, and modeled consumers before retrying".to_string(),
            );
            notes.push(
                "prefer refreshing or repairing the owning LUKS, LVM, VDO, multipath, cache, or filesystem layer before forcing generic map removal or rename retries".to_string(),
            );
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
        ) => {
            notes.push(
                "for filesystem changes, inspect mount state, source device signatures, usage, labels, UUIDs, and dependent services before retrying".to_string(),
            );
            notes.push(
                "prefer snapshots, read-only mounts, or cloned-device repair workflows before destructive format, shrink, repair, or device-removal retries".to_string(),
            );
        }
        (
            Operation::Create | Operation::Destroy | Operation::Login | Operation::Logout,
            Some("iscsiSessions"),
        ) => {
            notes.push(
                "for iSCSI session changes, inspect active sessions, node records, LUN paths, and multipath maps before retrying login or logout".to_string(),
            );
            notes.push(
                "keep dependent filesystems, LVM stacks, and services stopped or migrated until host-visible paths match the intended session state".to_string(),
            );
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
        ) => {
            notes.push(
                "for target-side LUN changes, inspect provider inventory, target mappings, host-visible SCSI paths, and multipath maps before retrying".to_string(),
            );
            notes.push(
                "stage host-side luns, iSCSI sessions, and multipath rescans only after the target reports the intended mapping and capacity".to_string(),
            );
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::Attach
            | Operation::Detach,
            Some("nvmeNamespaces"),
        ) => {
            notes.push(
                "for NVMe namespace changes, inspect namespace inventory and subsystem attachments before retrying create, grow/rescan, attach, detach, or delete operations".to_string(),
            );
            notes.push(
                "keep dependent filesystems, multipath maps, and consumers quiesced until namespace visibility and attachment state are verified".to_string(),
            );
        }
        (
            Operation::Create
            | Operation::Destroy
            | Operation::Grow
            | Operation::SetProperty
            | Operation::Start
            | Operation::Stop,
            Some("vdoVolumes"),
        ) => {
            notes.push(
                "for VDO lifecycle changes, inspect status, utilization, operating mode, and backing storage before retrying create, grow, start, stop, or removal".to_string(),
            );
            notes.push(
                "keep dependent filesystems, LVM layers, and services inactive until VDO mode and capacity match the intended topology".to_string(),
            );
        }
        (
            Operation::AddDevice
            | Operation::Destroy
            | Operation::Grow
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::Rescan,
            Some("multipathMaps"),
        ) => {
            notes.push(
                "for multipath changes, inspect path grouping, SCSI path state, map size, and modeled consumers before retrying reload, resize, path add, path removal, or flush operations".to_string(),
            );
            notes.push(
                "keep dependent filesystems, LVM layers, and services inactive or migrated until every expected path reports the intended map state".to_string(),
            );
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
        ) => {
            notes.push(
                "for cache changes, inspect dirty-data, cache mode, attachment, and backing volume state before retrying attach, detach, replacement, or property updates".to_string(),
            );
            notes.push(
                "prefer writethrough or clean-cache state before detaching, replacing, or disabling writeback cache layers".to_string(),
            );
        }
        (
            Operation::Create
            | Operation::Deactivate
            | Operation::Destroy
            | Operation::Format
            | Operation::Grow
            | Operation::Rescan
            | Operation::SetProperty,
            Some("swaps"),
        ) => {
            notes.push(
                "for swap changes, inspect active swapon output, signature metadata, resume references, and backing storage before retrying format, resize, property, or teardown operations".to_string(),
            );
            notes.push(
                "prefer adding temporary swap capacity before disabling or recreating active swap on memory-constrained systems".to_string(),
            );
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
        )
        | (
            Operation::AddKey
            | Operation::Create
            | Operation::Destroy
            | Operation::RemoveKey
            | Operation::SetProperty,
            Some("luksKeyslots"),
        )
        | (
            Operation::Create
            | Operation::Destroy
            | Operation::ImportToken
            | Operation::RemoveToken
            | Operation::SetProperty,
            Some("luksTokens"),
        ) => {
            notes.push(
                "for LUKS changes, inspect mapper status, header metadata, keyslots, tokens, and dependent consumers before retrying encryption operations".to_string(),
            );
            notes.push(
                "keep header backups and alternate unlock paths available until the mapper and header metadata match the intended state".to_string(),
            );
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
        ) => {
            notes.push(
                "for LVM changes, inspect LV, PV, and VG metadata before retrying activation, resize, rename, import, export, create, or removal operations".to_string(),
            );
            notes.push(
                "keep dependent filesystems, encryption layers, and services inactive until LVM metadata and activation state match the intended topology".to_string(),
            );
        }
        (
            Operation::Stop
            | Operation::RemoveDevice
            | Operation::ReplaceDevice
            | Operation::AddDevice,
            Some("mdRaids"),
        ) => {
            notes.push(
                "for MD RAID member changes, inspect mdadm detail and /proc/mdstat before retrying; do not remove old members until sync or replacement state is understood".to_string(),
            );
            notes.push(
                "keep failed, old, and replacement devices attached until redundancy and array metadata are verified".to_string(),
            );
        }
        (Operation::RemoveDevice | Operation::Destroy | Operation::Detach, _) => {
            notes.push(
                "verify consumers, redundancy, and metadata health before retrying teardown or device removal".to_string(),
            );
            notes.push(
                "prefer roll-forward repair of the partially changed topology over blind rollback when data placement may have moved".to_string(),
            );
        }
        _ => {
            notes.push(
                "choose rollback only when domain-specific tooling proves it is safer than completing the remaining plan".to_string(),
            );
        }
    }
    notes
}

fn command_step_collection(step: &ExecutionStep) -> Option<&str> {
    step.action_id
        .split(':')
        .next()
        .map(|collection| match collection {
            "snapshot" => "snapshots",
            "filesystem" => "filesystems",
            "backingfiles" => "backingFiles",
            "btrfsqgroups" => "btrfsQgroups",
            "btrfssubvolumes" => "btrfsSubvolumes",
            "dmmaps" => "dmMaps",
            "iscsisessions" => "iscsiSessions",
            "loopdevices" => "loopDevices",
            "lvmcaches" => "lvmCaches",
            "lukskeyslots" => "luksKeyslots",
            "lukstokens" => "luksTokens",
            "multipathmaps" => "multipathMaps",
            "nvmenamespaces" => "nvmeNamespaces",
            "physicalvolumes" => "physicalVolumes",
            "targetLuns" | "targetluns" => "targetLuns",
            "thinpools" => "thinPools",
            "volumegroups" => "volumeGroups",
            "vdovolumes" => "vdoVolumes",
            "zvols" => "zvols",
            other => other,
        })
}

fn command_step_target(step: &ExecutionStep) -> Option<&str> {
    if command_step_collection(step) == Some("mdRaids") {
        if let Some(target) = md_array_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("nvmeNamespaces") {
        if let Some(target) = nvme_controller_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("iscsiSessions") {
        if let Some(target) = iscsi_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("targetLuns") {
        if let Some(target) = target_lun_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("multipathMaps") {
        if let Some(target) = multipath_map_target_from_step(step) {
            return Some(target);
        }
    }
    if matches!(
        command_step_collection(step),
        Some("volumes" | "thinPools" | "physicalVolumes" | "volumeGroups")
    ) {
        if let Some(target) = lvm_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("snapshots") {
        if let Some(target) = snapshot_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("btrfsSubvolumes") {
        if let Some(target) = btrfs_subvolume_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("btrfsQgroups") {
        if let Some(target) = btrfs_qgroup_target_from_step(step) {
            return Some(target);
        }
    }
    if matches!(command_step_collection(step), Some("caches" | "lvmCaches")) {
        if let Some(target) = cache_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("swaps") {
        if let Some(target) = swap_target_from_step(step) {
            return Some(target);
        }
    }
    if matches!(
        command_step_collection(step),
        Some("pools" | "datasets" | "zvols")
    ) {
        if let Some(target) = zfs_target_from_step(step) {
            return Some(target);
        }
    }
    if command_step_collection(step) == Some("filesystems") {
        if let Some(target) = filesystem_target_from_step(step) {
            return Some(target);
        }
    }
    if matches!(command_step_collection(step), Some("disks" | "partitions")) {
        if let Some(target) =
            partition_disk_from_step(step).or_else(|| partition_target_from_step(step))
        {
            return Some(target);
        }
    }
    if matches!(
        command_step_collection(step),
        Some("backingFiles" | "loopDevices" | "dmMaps")
    ) {
        if let Some(target) = local_mapping_target_from_step(step) {
            return Some(target);
        }
    }
    if matches!(
        command_step_collection(step),
        Some("exports" | "nfs.mounts")
    ) {
        if let Some(target) = nfs_target_from_step(step) {
            return Some(target);
        }
    }
    step.action_id
        .split(':')
        .nth(1)
        .filter(|target| !target.is_empty())
}

fn md_array_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_some_and(|arg| arg == "mdadm") {
            if command
                .argv
                .get(1)
                .is_some_and(|arg| arg == "--detail" || arg == "--stop")
            {
                return command.argv.get(2).map(String::as_str);
            }
            if command
                .argv
                .get(1)
                .is_some_and(|arg| arg.starts_with("/dev/md"))
            {
                return command.argv.get(1).map(String::as_str);
            }
        }
        None
    })
}

fn multipath_map_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_some_and(|arg| arg == "multipath")
            && command
                .argv
                .get(1)
                .is_some_and(|arg| arg == "-ll" || arg == "-f")
        {
            return command.argv.get(2).map(String::as_str);
        }
        if command.argv.first().is_some_and(|arg| arg == "multipathd")
            && command
                .argv
                .get(1..3)
                .is_some_and(|args| args == ["resize", "map"])
        {
            return command.argv.get(3).map(String::as_str);
        }
        None
    })
}
