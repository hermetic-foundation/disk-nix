fn luks_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    if let Some(device) = luks_device_from_step(step) {
        commands.push(command(["cryptsetup", "luksDump", device], false, note));
        commands.push(command(
            ["disk-nix", "inspect", device, "--json"],
            false,
            note,
        ));
    }
    if let Some(mapper) = luks_mapper_from_step(step) {
        commands.push(command(["cryptsetup", "status", mapper], false, note));
        commands.push(command(
            ["disk-nix", "inspect", mapper, "--json"],
            false,
            note,
        ));
    }
    commands
}

fn luks_mapper_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_none_or(|arg| arg != "cryptsetup") {
            return None;
        }
        match command.argv.get(1).map(String::as_str) {
            Some("close" | "resize" | "status") => command.argv.get(2).map(String::as_str),
            Some("open") => command.argv.get(3).map(String::as_str),
            _ => None,
        }
    })
}

fn luks_device_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_none_or(|arg| arg != "cryptsetup") {
            return None;
        }
        match command.argv.get(1).map(String::as_str) {
            Some("isLuks" | "luksDump" | "luksFormat" | "luksKillSlot" | "luksUUID") => {
                command.argv.get(2).map(String::as_str)
            }
            Some("open") => command.argv.get(2).map(String::as_str),
            Some("config" | "luksAddKey" | "luksChangeKey") => {
                cryptsetup_positional_arg(command, 0)
            }
            Some("token") => command.argv.last().map(String::as_str),
            _ => None,
        }
        .filter(|target| !target.starts_with('<'))
    })
}

fn cryptsetup_positional_arg(command: &ExecutionCommand, index: usize) -> Option<&str> {
    let mut skip_next = false;
    let mut position = 0;
    for arg in command.argv.iter().skip(2) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if matches!(
            arg.as_str(),
            "--key-file"
                | "--key-slot"
                | "--json-file"
                | "--priority"
                | "--subsystem"
                | "--token-id"
                | "--uuid"
        ) {
            skip_next = true;
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        if position == index {
            return Some(arg.as_str());
        }
        position += 1;
    }
    None
}

fn lvm_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    match command_step_collection(step) {
        Some("physicalVolumes") => {
            if let Some(target) = lvm_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| !target.is_empty())
            }) {
                commands.push(command(
                    ["pvs", "--reportformat", "json", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["vgs", "--reportformat", "json"], false, note));
            commands.push(command(["lvs", "--reportformat", "json"], false, note));
        }
        Some("volumes" | "thinPools") => {
            if let Some(target) = lvm_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| !target.is_empty())
            }) {
                commands.push(command(
                    ["lvs", "--reportformat", "json", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["vgs", "--reportformat", "json"], false, note));
            commands.push(command(["pvs", "--reportformat", "json"], false, note));
        }
        Some("volumeGroups") => {
            if let Some(target) = lvm_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| !target.is_empty())
            }) {
                commands.push(command(
                    ["vgs", "--reportformat", "json", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["pvs", "--reportformat", "json"], false, note));
            commands.push(command(
                ["lvs", "--reportformat", "json", "-a"],
                false,
                note,
            ));
        }
        _ => {}
    }
    commands
}

fn lvm_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        match tool {
            "lvs" | "pvs" | "vgs" if command.argv.len() > 3 => {
                command.argv.last().map(String::as_str)
            }
            "lvchange" | "lvextend" | "lvremove" | "lvreduce" => {
                command.argv.last().map(String::as_str)
            }
            "lvrename" => command.argv.get(1).map(String::as_str),
            "pvcreate" | "pvremove" | "pvresize" => command.argv.last().map(String::as_str),
            "pvscan" if command.argv.len() > 2 => command.argv.last().map(String::as_str),
            "vgchange" | "vgexport" | "vgimport" | "vgremove" => {
                command.argv.last().map(String::as_str)
            }
            "vgcreate" | "vgextend" | "vgreduce" | "vgrename" => {
                command.argv.get(1).map(String::as_str)
            }
            _ => None,
        }
        .filter(|target| !target.starts_with('<'))
    })
}

fn cache_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    let target = cache_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| !target.is_empty())
    });

    match command_step_collection(step) {
        Some("lvmCaches") => {
            if let Some(target) = target {
                commands.push(command(
                    [
                        "lvs",
                        "--reportformat",
                        "json",
                        "-a",
                        "-o",
                        "lv_name,lv_attr,origin,cache_mode,cache_policy,data_percent,metadata_percent",
                        target,
                    ],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["vgs", "--reportformat", "json"], false, note));
            commands.push(command(["pvs", "--reportformat", "json"], false, note));
        }
        Some("caches") => {
            if let Some(target) = target {
                commands.push(bcache_sysfs_read_command(target, "state", note));
                commands.push(bcache_sysfs_read_command(target, "cache_mode", note));
                commands.push(bcache_sysfs_read_command(target, "dirty_data", note));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            }
            commands.push(command(["disk-nix", "cache", "--json"], false, note));
        }
        _ => {}
    }
    commands
}

fn cache_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        match tool {
            "sh" => command
                .argv
                .get(3)
                .filter(|wrapper| wrapper.starts_with("disk-nix-bcache-"))
                .and_then(|_| command.argv.get(4))
                .map(String::as_str),
            "lvchange" | "lvconvert" => command.argv.last().map(String::as_str),
            "lvs" if command.argv.len() > 3 => command.argv.last().map(String::as_str),
            _ => None,
        }
        .filter(|target| !target.starts_with('<'))
    })
}

fn swap_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = vec![command(
        ["swapon", "--show", "--bytes", "--raw"],
        false,
        note,
    )];
    if let Some(target) = swap_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| target.starts_with('/'))
    }) {
        commands.push(command(["blkid", target], false, note));
        commands.push(command(
            ["disk-nix", "inspect", target, "--json"],
            false,
            note,
        ));
    } else {
        commands.push(command(
            ["disk-nix", "swap", "--json"],
            false,
            "inspect modeled swap inventory before retrying",
        ));
    }
    commands
}

fn swap_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        match tool {
            "fallocate" | "mkswap" | "swaplabel" | "swapoff" | "wipefs" => {
                command.argv.last().map(String::as_str)
            }
            "swapon" if command.argv.get(1).is_none_or(|arg| arg != "--show") => {
                command.argv.last().map(String::as_str)
            }
            "sh" if command.argv.get(1).is_some_and(|arg| arg == "-c") => {
                swap_target_from_shell(command.argv.get(2)?)
            }
            _ => None,
        }
        .filter(|target| target.starts_with('/') && !target.starts_with("<"))
    })
}

fn swap_target_from_shell(script: &str) -> Option<&str> {
    let target = script.strip_prefix("swapoff ")?.split_whitespace().next()?;
    target
        .trim_matches('\'')
        .trim_matches('"')
        .strip_prefix("\\")
        .or(Some(target.trim_matches('\'').trim_matches('"')))
}

fn zfs_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let target = zfs_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| !target.is_empty())
    });
    let mut commands = Vec::new();

    match command_step_collection(step) {
        Some("pools") => {
            if let Some(target) = target {
                commands.push(command(["zpool", "status", "-P", target], false, note));
                commands.push(command(["zpool", "list", "-H", "-p", target], false, note));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(["zpool", "status", "-P"], false, note));
                commands.push(command(["zpool", "list", "-H", "-p"], false, note));
            }
        }
        Some("datasets") => {
            if let Some(target) = target {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", "-t", "filesystem", target],
                    false,
                    note,
                ));
                commands.push(command(["zfs", "get", "all", target], false, note));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", "-t", "filesystem"],
                    false,
                    note,
                ));
            }
        }
        Some("zvols") => {
            if let Some(target) = target {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", "-t", "volume", target],
                    false,
                    note,
                ));
                commands.push(command(["zfs", "get", "all", target], false, note));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["zfs", "list", "-H", "-p", "-t", "volume"],
                    false,
                    note,
                ));
            }
        }
        _ => {}
    }

    commands
}

fn zfs_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "zpool" => match command.argv.get(1).map(String::as_str) {
                Some("add" | "create" | "destroy" | "export" | "remove" | "replace" | "scrub") => {
                    command.argv.get(2).map(String::as_str)
                }
                Some("import") => command.argv.last().map(String::as_str),
                Some("set") => command.argv.get(3).map(String::as_str),
                Some("list" | "status" | "get") if command.argv.len() > 3 => {
                    command.argv.last().map(String::as_str)
                }
                _ => None,
            },
            "zfs" => match command.argv.get(1).map(String::as_str) {
                Some("create" | "destroy" | "get" | "promote" | "set") => {
                    command.argv.last().map(String::as_str)
                }
                Some("rename") => command.argv.get(2).map(String::as_str),
                Some("list") if command.argv.len() > 4 => command.argv.last().map(String::as_str),
                _ => None,
            },
            _ => None,
        }?;

        Some(target).filter(|target| {
            !target.is_empty()
                && !target.starts_with('-')
                && !target.starts_with('<')
                && *target != "import"
        })
    })
}

fn filesystem_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let target = filesystem_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| !target.is_empty())
    });
    let source = filesystem_source_from_step(step);
    let mut commands = Vec::new();

    if let Some(mountpoint) = target.filter(|target| target.starts_with('/')) {
        commands.push(command(
            ["findmnt", "--json", "--target", mountpoint],
            false,
            note,
        ));
        commands.push(command(
            ["disk-nix", "inspect", mountpoint, "--json"],
            false,
            note,
        ));
    }

    if let Some(source) = source {
        commands.push(command(["blkid", source], false, note));
        commands.push(command(
            ["disk-nix", "inspect", source, "--json"],
            false,
            note,
        ));
    }

    if commands.is_empty() {
        commands.push(command(
            ["disk-nix", "filesystems", "--json"],
            false,
            "inspect modeled filesystem inventory before retrying",
        ));
    }

    commands
}

fn filesystem_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "xfs_growfs" | "fstrim" | "umount" => command.argv.get(1).map(String::as_str),
            "mount" => command.argv.last().map(String::as_str),
            "findmnt" if command.argv.iter().any(|arg| arg == "--target") => {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["filesystem", "resize"]) =>
            {
                command.argv.get(3).map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["filesystem", "usage"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["balance", "start"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["scrub", "start"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "zfs" if command.argv.get(1).is_some_and(|arg| arg == "set") => {
                command.argv.last().map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| {
            !target.is_empty() && !target.starts_with('-') && !target.starts_with('<')
        })
    })
}

fn filesystem_source_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let source = match tool {
            "blkid" => command.argv.get(1).map(String::as_str),
            "mount" if command.argv.len() >= 3 => {
                command.argv.get(command.argv.len() - 2).map(String::as_str)
            }
            "resize2fs" | "resize.f2fs" | "e2fsck" | "xfs_repair" | "ntfsfix" => {
                command.argv.last().map(String::as_str)
            }
            "fsck.fat" | "fsck.exfat" | "fsck.f2fs" => command.argv.last().map(String::as_str),
            "btrfs" if command.argv.get(1).is_some_and(|arg| arg == "check") => {
                command.argv.last().map(String::as_str)
            }
            "bcachefs"
                if command
                    .argv
                    .get(1)
                    .is_some_and(|arg| arg == "fsck" || arg == "format") =>
            {
                command.argv.last().map(String::as_str)
            }
            "bcachefs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["device", "resize"]) =>
            {
                command.argv.get(3).map(String::as_str)
            }
            tool if tool.starts_with("mkfs.") => command.argv.last().map(String::as_str),
            "mkfs" => command.argv.last().map(String::as_str),
            "e2label" | "fatlabel" | "ntfslabel" | "exfatlabel" | "f2fslabel" => {
                command.argv.get(1).map(String::as_str)
            }
            "xfs_admin" => command.argv.last().map(String::as_str),
            _ => None,
        }?;

        Some(source)
            .filter(|source| source.starts_with('/') && !source.starts_with("<") && *source != "/")
    })
}

fn partition_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let disk = partition_disk_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| target.starts_with('/'))
    });
    let partition = partition_target_from_step(step);
    let mut commands = Vec::new();

    if let Some(disk) = disk {
        commands.push(command(["parted", "-lm", disk], false, note));
        commands.push(command(
            ["lsblk", "--json", "--bytes", "--output-all", disk],
            false,
            note,
        ));
        commands.push(command(
            ["disk-nix", "inspect", disk, "--json"],
            false,
            note,
        ));
    } else {
        commands.push(command(
            ["parted", "-lm"],
            false,
            "inspect all partition tables before retrying",
        ));
        commands.push(command(
            ["lsblk", "--json", "--bytes", "--output-all"],
            false,
            "inspect kernel disk and partition inventory before retrying",
        ));
    }

    if let Some(partition) = partition {
        commands.push(command(
            ["disk-nix", "inspect", partition, "--json"],
            false,
            note,
        ));
    }

    commands
}

fn partition_disk_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let disk = match tool {
            "parted" => match command.argv.get(1).map(String::as_str) {
                Some("-s") => command.argv.get(2).map(String::as_str),
                Some("-lm") => command.argv.get(2).map(String::as_str),
                _ => command.argv.last().map(String::as_str),
            },
            "partprobe" => command.argv.get(1).map(String::as_str),
            "blockdev" if command.argv.get(1).is_some_and(|arg| arg == "--rereadpt") => {
                command.argv.get(2).map(String::as_str)
            }
            "growpart" => command.argv.get(1).map(String::as_str),
            _ => None,
        }?;

        Some(disk).filter(|disk| {
            disk.starts_with('/') && !disk.starts_with('<') && !disk.starts_with('-')
        })
    })
}

fn partition_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command
            .argv
            .get(0..2)
            .is_some_and(|args| args == ["disk-nix", "inspect"])
        {
            return command
                .argv
                .get(2)
                .map(String::as_str)
                .filter(|target| target.starts_with('/') && !target.starts_with('<'));
        }
        None
    })
}

fn nfs_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    match command_step_collection(step) {
        Some("exports") => {
            let target = nfs_export_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| target.starts_with('/'))
            });
            let mut commands = vec![command(["exportfs", "-v"], false, note)];
            if let Some(target) = target {
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["disk-nix", "nfs", "--json"],
                    false,
                    "inspect modeled NFS exports before retrying",
                ));
            }
            commands
        }
        Some("nfs.mounts") => {
            let mountpoint = nfs_mount_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| target.starts_with('/'))
            });
            let mut commands = Vec::new();
            if let Some(mountpoint) = mountpoint {
                commands.push(command(["findmnt", "--json", mountpoint], false, note));
                commands.push(command(["nfsstat", "-m", mountpoint], false, note));
                commands.push(command(
                    ["disk-nix", "inspect", mountpoint, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["findmnt", "--json", "--types", "nfs,nfs4"],
                    false,
                    "inspect active NFS mounts before retrying",
                ));
                commands.push(command(
                    ["disk-nix", "nfs", "--json"],
                    false,
                    "inspect modeled NFS mounts before retrying",
                ));
            }
            commands
        }
        _ => Vec::new(),
    }
}

fn nfs_target_from_step(step: &ExecutionStep) -> Option<&str> {
    nfs_export_target_from_step(step).or_else(|| nfs_mount_target_from_step(step))
}

fn nfs_export_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_none_or(|arg| arg != "exportfs") {
            return None;
        }
        command
            .argv
            .last()
            .and_then(|target| {
                target
                    .split_once(':')
                    .map(|(_, path)| path)
                    .or(Some(target))
            })
            .filter(|target| target.starts_with('/') && !target.starts_with('<'))
    })
}

fn nfs_mount_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "mount" | "umount" | "findmnt" | "nfsstat" => command.argv.last().map(String::as_str),
            "disk-nix" if command.argv.get(1).is_some_and(|arg| arg == "inspect") => {
                command.argv.get(2).map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| target.starts_with('/') && !target.starts_with('<'))
    })
}

fn local_mapping_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    match command_step_collection(step) {
        Some("dmMaps") => {
            let target = dm_map_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| is_dm_map_target(target))
            });
            vec![
                dmsetup_info_command(target, note),
                dmsetup_deps_command(target),
                dmsetup_table_command(target),
                dmsetup_status_command(target),
                dm_map_inspect_json_command(target, note),
            ]
        }
        Some("loopDevices") => {
            let target = loop_target_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| target.starts_with("/dev/loop"))
            });
            let mut commands = Vec::new();
            if let Some(target) = target {
                commands.push(command(
                    ["losetup", "--json", "--list", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["losetup", "--json", "--list"],
                    false,
                    "inspect loop mappings before retrying",
                ));
            }
            if let Some(backing) = backing_file_from_step(step) {
                commands.push(command(
                    ["stat", "--printf=%n %s %b %B\\n", backing],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", backing, "--json"],
                    false,
                    note,
                ));
            }
            commands
        }
        Some("backingFiles") => {
            let target = backing_file_from_step(step).or_else(|| {
                step.action_id
                    .split(':')
                    .nth(1)
                    .filter(|target| target.starts_with('/'))
            });
            let mut commands = Vec::new();
            if let Some(target) = target {
                commands.push(command(
                    ["stat", "--printf=%n %s %b %B\\n", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["du", "--bytes", "--apparent-size", target],
                    false,
                    note,
                ));
                commands.push(command(
                    ["disk-nix", "inspect", target, "--json"],
                    false,
                    note,
                ));
            } else {
                commands.push(command(
                    ["disk-nix", "backing-files", "--json"],
                    false,
                    "inspect modeled backing-file inventory before retrying",
                ));
            }
            commands
        }
        _ => Vec::new(),
    }
}

fn local_mapping_target_from_step(step: &ExecutionStep) -> Option<&str> {
    dm_map_target_from_step(step)
        .or_else(|| loop_target_from_step(step))
        .or_else(|| backing_file_from_step(step))
}

fn dm_map_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "dmsetup" => match command.argv.get(1).map(String::as_str) {
                Some("rename") => command.argv.get(2).map(String::as_str),
                Some("remove" | "deps" | "table" | "status") => {
                    command.argv.get(2).map(String::as_str)
                }
                Some("info") => command.argv.last().map(String::as_str),
                _ => None,
            },
            "disk-nix" if command.argv.get(1).is_some_and(|arg| arg == "inspect") => {
                command.argv.get(2).map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| is_dm_map_target(target))
    })
}

fn loop_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "losetup" => match command.argv.get(1).map(String::as_str) {
                Some("--detach" | "-c") => command.argv.get(2).map(String::as_str),
                Some("--json") => command.argv.last().map(String::as_str),
                Some(target) if target.starts_with("/dev/loop") => {
                    command.argv.get(1).map(String::as_str)
                }
                _ => None,
            },
            "disk-nix" if command.argv.get(1).is_some_and(|arg| arg == "inspect") => {
                command.argv.get(2).map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| target.starts_with("/dev/loop") && !target.starts_with('<'))
    })
}

fn backing_file_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        let target = match tool {
            "truncate" | "stat" | "du" | "test" => command.argv.last().map(String::as_str),
            "losetup" => command.argv.last().map(String::as_str),
            "disk-nix" if command.argv.get(1).is_some_and(|arg| arg == "inspect") => {
                command.argv.get(2).map(String::as_str)
            }
            _ => None,
        }?;

        Some(target).filter(|target| {
            target.starts_with('/')
                && !target.starts_with('<')
                && !target.starts_with("/dev/loop")
                && !is_dm_map_target(target)
        })
    })
}

fn snapshot_recovery_inspection_commands(
    step: &ExecutionStep,
    note: &'static str,
) -> Vec<ExecutionCommand> {
    let mut commands = Vec::new();
    if let Some(snapshot) = snapshot_target_from_step(step).or_else(|| {
        step.action_id
            .split(':')
            .nth(1)
            .filter(|target| !target.is_empty())
    }) {
        if is_zfs_snapshot_name(snapshot) {
            commands.push(command(
                ["zfs", "list", "-t", "snapshot", "-H", "-p", snapshot],
                false,
                note,
            ));
            commands.push(command(["zfs", "holds", snapshot], false, note));
            if let Some(dataset) = zfs_snapshot_dataset(snapshot) {
                commands.push(command(["zfs", "list", "-H", "-p", dataset], false, note));
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
                    note,
                ));
            }
            commands.push(command(
                ["disk-nix", "inspect", snapshot, "--json"],
                false,
                note,
            ));
        } else if snapshot.starts_with('/') {
            commands.push(command(
                ["btrfs", "subvolume", "show", snapshot],
                false,
                note,
            ));
            commands.push(command(
                ["btrfs", "property", "get", "-ts", snapshot, "ro"],
                false,
                note,
            ));
            commands.push(command(
                ["disk-nix", "inspect", snapshot, "--json"],
                false,
                note,
            ));
        } else {
            commands.push(command(
                ["disk-nix", "inspect", snapshot, "--json"],
                false,
                note,
            ));
        }
    }
    commands
}

fn snapshot_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        let tool = command.argv.first()?.as_str();
        match tool {
            "zfs" => match command.argv.get(1).map(String::as_str) {
                Some("snapshot" | "destroy" | "rollback" | "holds") => {
                    command.argv.last().map(String::as_str)
                }
                Some("clone" | "rename") => command.argv.get(2).map(String::as_str),
                Some("hold" | "release") => command.argv.last().map(String::as_str),
                Some("list")
                    if command
                        .argv
                        .iter()
                        .any(|arg| arg == "-t" || arg == "snapshot") =>
                {
                    command.argv.last().map(String::as_str)
                }
                _ => None,
            },
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["subvolume", "show"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["subvolume", "delete"]) =>
            {
                command.argv.last().map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["subvolume", "snapshot"]) =>
            {
                command
                    .argv
                    .iter()
                    .skip(3)
                    .find(|arg| !arg.starts_with('-'))
                    .map(String::as_str)
            }
            "btrfs"
                if command
                    .argv
                    .get(1..3)
                    .is_some_and(|args| args == ["property", "get"]) =>
            {
                command
                    .argv
                    .iter()
                    .skip(3)
                    .find(|arg| arg.starts_with('/'))
                    .map(String::as_str)
            }
            "mv" => command.argv.iter().skip(1).find_map(|arg| {
                if arg == "--" || arg.starts_with('-') {
                    None
                } else {
                    Some(arg.as_str())
                }
            }),
            _ => None,
        }
        .filter(|target| !target.starts_with('<'))
    })
}

fn btrfs_subvolume_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command
            .argv
            .get(0..3)
            .is_some_and(|args| args == ["btrfs", "subvolume", "show"])
            || command
                .argv
                .get(0..3)
                .is_some_and(|args| args == ["btrfs", "subvolume", "create"])
            || command
                .argv
                .get(0..3)
                .is_some_and(|args| args == ["btrfs", "subvolume", "delete"])
        {
            return command.argv.get(3).map(String::as_str);
        }
        if command
            .argv
            .get(0..4)
            .is_some_and(|args| args == ["btrfs", "property", "set", "-ts"])
            || command
                .argv
                .get(0..4)
                .is_some_and(|args| args == ["btrfs", "property", "get", "-ts"])
        {
            return command.argv.get(4).map(String::as_str);
        }
        if command
            .argv
            .get(0..2)
            .is_some_and(|args| args == ["mv", "--"])
        {
            return command.argv.get(2).map(String::as_str);
        }
        None
    })
}

fn btrfs_qgroup_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command
            .argv
            .get(0..5)
            .is_some_and(|args| args == ["btrfs", "qgroup", "show", "--raw", "-reF"])
        {
            return command.argv.get(5).map(String::as_str);
        }
        if command
            .argv
            .get(0..3)
            .is_some_and(|args| args == ["btrfs", "qgroup", "create"])
            || command
                .argv
                .get(0..3)
                .is_some_and(|args| args == ["btrfs", "qgroup", "destroy"])
        {
            return command.argv.get(4).map(String::as_str);
        }
        if command
            .argv
            .get(0..3)
            .is_some_and(|args| args == ["btrfs", "qgroup", "limit"])
        {
            return if command.argv.get(3).is_some_and(|arg| arg == "-e") {
                command.argv.get(6).map(String::as_str)
            } else {
                command.argv.get(5).map(String::as_str)
            };
        }
        None
    })
}

fn iscsi_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_some_and(|arg| arg == "iscsiadm") {
            return command
                .argv
                .windows(2)
                .find(|window| window[0] == "--targetname")
                .map(|window| window[1].as_str());
        }
        None
    })
}

fn nvme_controller_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.commands.iter().find_map(|command| {
        if command.argv.first().is_some_and(|arg| arg == "nvme")
            && command.argv.get(1).is_some_and(|arg| {
                matches!(
                    arg.as_str(),
                    "attach-ns" | "create-ns" | "delete-ns" | "detach-ns" | "list-ns" | "ns-rescan"
                )
            })
        {
            return command
                .argv
                .get(2)
                .map(String::as_str)
                .filter(|target| is_nvme_controller_path(target));
        }
        None
    })
}

fn target_lun_target_from_step(step: &ExecutionStep) -> Option<&str> {
    step.action_id
        .strip_prefix("targetluns:")
        .or_else(|| step.action_id.strip_prefix("targetLuns:"))
        .and_then(|rest| {
            rest.split_once(":set-property:")
                .map(|(target, _)| target)
                .or_else(|| rest.rsplit_once(':').map(|(target, _)| target))
        })
        .filter(|target| !target.is_empty())
}

fn failed_result_notes(result: &ExecutionCommandResult) -> Vec<String> {
    let mut notes = vec![
        format!(
            "{:?} phase failed for action {}",
            result.phase, result.action_id
        ),
        format!("command: {}", result.argv.join(" ")),
    ];
    if let Some(status_code) = result.status_code {
        notes.push(format!("exit status: {status_code}"));
    }
    if !result.stderr.trim().is_empty() {
        notes.push(format!("stderr: {}", result.stderr.trim()));
    }
    notes
}

fn target_lun_recovery_inspection_commands(
    target: Option<&str>,
    note: &str,
) -> Vec<ExecutionCommand> {
    let mut commands = vec![
        command_vec(["targetcli", "/iscsi", "ls"], false, note),
        command_vec(
            [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show",
            ],
            false,
            note,
        ),
        lsscsi_lun_inventory_command(note),
        command_vec(["multipath", "-ll"], false, note),
    ];
    if let Some(target) = target {
        commands.insert(
            1,
            command_vec(
                vec![
                    "targetcli".to_string(),
                    format!("/iscsi/{target}"),
                    "ls".to_string(),
                ],
                false,
                note,
            ),
        );
    }
    commands
}
